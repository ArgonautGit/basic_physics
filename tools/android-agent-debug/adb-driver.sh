#!/usr/bin/env bash
# Agentic debug driver for an Android app running on a headless emulator (or any
# adb-connected ARM device). The analog of the egui debug_driver: see the screen,
# inspect the view tree, and interact — all from a script an agent can generate.
#
# Interaction is against the LIVE device (state persists between runs), unlike the
# egui driver's fresh-replay model.
#
# Usage:
#   tools/android-agent-debug/adb-driver.sh script.txt
#   printf 'tree\ntap-text "OK"\nscreenshot shots/after.png\n' | tools/android-agent-debug/adb-driver.sh
#
# Commands (one per line; '#' comments and blank lines ignored; quote args with spaces):
#   screenshot <path>        save a PNG of the current screen (screencap)
#   tree                     print a compact view hierarchy (uiautomator dump)
#   tap <x> <y>              tap raw screen coordinates
#   tap-text <substring>     tap the center of the first node whose text/desc contains this
#   tap-id <resource-id>     tap the center of the node with this resource-id (full or suffix)
#   text <string>            type into the focused field
#   key <name>               key event: enter back home tab del space up down left right menu app_switch
#   swipe <x1> <y1> <x2> <y2> [ms]   swipe gesture
#   launch <pkg>[/<activity>]   start an app (monkey if no activity given)
#   stop <pkg>               force-stop an app
#   install <apk>            install (or reinstall) an APK
#   state                    print current focused activity + device props
#   wait <ms>                sleep
#   shell <cmd...>           run an arbitrary adb shell command (escape hatch)
#
# Each command prints 'OK ...' or 'ERR ...'; exit code is 1 if any command failed.
set -uo pipefail

export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/android-sdk}"
export PATH="$ANDROID_SDK_ROOT/platform-tools:$ANDROID_SDK_ROOT/emulator:$PATH"
ADB="${ADB:-adb}"
SERIAL="${ANDROID_SERIAL:-}"
# Always give adb an empty stdin: `adb shell`/`adb exec-out` read stdin, which
# would otherwise swallow the rest of a piped command script.
adb_() { if [ -n "$SERIAL" ]; then "$ADB" -s "$SERIAL" "$@" </dev/null; else "$ADB" "$@" </dev/null; fi; }

fail=0
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ---- helpers ---------------------------------------------------------------

# Dump the current window to XML at $TMP/window.xml (uiautomator). Returns nonzero
# on failure (nothing focusable, dump busy, or the call exceeds DUMP_TIMEOUT).
# uiautomator can be slow or get OOM-killed on a heavily loaded software emulator,
# so we bound it rather than let `tree`/`tap-*` hang.
DUMP_TIMEOUT="${DUMP_TIMEOUT:-45}"
dump_xml() {
  local -a base=("$ADB")
  [ -n "$SERIAL" ] && base+=(-s "$SERIAL")
  timeout "$DUMP_TIMEOUT" "${base[@]}" exec-out uiautomator dump /dev/tty </dev/null 2>/dev/null \
    | sed 's/UI hierchary dumped to: \/dev\/tty//' > "$TMP/window.xml"
  # 124 == timed out: the device is wedged, don't pile on a second slow attempt.
  [ "${PIPESTATUS[0]}" = 124 ] && return 1
  grep -q '<hierarchy' "$TMP/window.xml" 2>/dev/null && return 0
  # Some devices only support dumping to a file, not /dev/tty.
  timeout "$DUMP_TIMEOUT" "${base[@]}" shell uiautomator dump /sdcard/window_dump.xml </dev/null >/dev/null 2>&1 || return 1
  adb_ exec-out cat /sdcard/window_dump.xml > "$TMP/window.xml" 2>/dev/null || return 1
  grep -q '<hierarchy' "$TMP/window.xml"
}

# Print a compact, indented view of the hierarchy (interactive/labelled nodes).
print_tree() {
  python3 - "$TMP/window.xml" <<'PY'
import sys, xml.etree.ElementTree as ET
try:
    root = ET.parse(sys.argv[1]).getroot()
except Exception as e:
    print(f"ERR could not parse dump: {e}"); sys.exit(2)
def short(cls): return cls.rsplit('.', 1)[-1] if cls else cls
def walk(n, depth):
    a = n.attrib
    parts = [short(a.get('class',''))]
    for k in ('text','content-desc','resource-id'):
        v = a.get(k)
        if v: parts.append(f'{k.split("-")[0]}={v!r}')
    for k in ('clickable','checkable','scrollable'):
        if a.get(k) == 'true': parts.append(k)
    b = a.get('bounds')
    if b: parts.append(b)
    # Only print rows that carry something an agent can target or read.
    interesting = any(a.get(k) for k in ('text','content-desc','resource-id')) \
        or a.get('clickable') == 'true' or len(n) == 0
    if interesting:
        print('  '*depth + ' '.join(parts))
    for c in n:
        walk(c, depth+1)
for c in root:
    walk(c, 0)
PY
}

keycode() {
  case "$1" in
    enter) echo 66;; back) echo 4;; home) echo 3;; tab) echo 61;;
    del|backspace) echo 67;; space) echo 62;; up) echo 19;; down) echo 20;;
    left) echo 21;; right) echo 22;; menu) echo 82;; app_switch) echo 187;;
    power) echo 26;; *) echo "$1";;  # allow a raw numeric keycode
  esac
}

run_cmd() {
  # shellcheck disable=SC2086
  local cmd="$1"; shift
  case "$cmd" in
    screenshot)
      local out="$1"; [ -n "$out" ] || { echo "ERR screenshot: need <path>"; return 1; }
      mkdir -p "$(dirname "$out")" 2>/dev/null
      if adb_ exec-out screencap -p > "$out" 2>/dev/null && [ -s "$out" ]; then
        echo "OK screenshot $out"
      else echo "ERR screenshot: screencap failed"; return 1; fi;;
    tree)
      if dump_xml; then print_tree; echo "OK tree"; else echo "ERR tree: uiautomator dump failed"; return 1; fi;;
    tap)
      [ $# -ge 2 ] || { echo "ERR tap: need <x> <y>"; return 1; }
      adb_ shell input tap "$1" "$2" && echo "OK tap $1 $2" || { echo "ERR tap failed"; return 1; };;
    tap-text)
      local sub="$1"; dump_xml || { echo "ERR tap-text: dump failed"; return 1; }
      local xy; xy="$(match_center "$sub" text)" || { echo "ERR tap-text: no node containing ${sub@Q}"; return 1; }
      adb_ shell input tap $xy && echo "OK tap-text ${sub@Q} @ $xy" || { echo "ERR tap-text failed"; return 1; };;
    tap-id)
      local rid="$1"; dump_xml || { echo "ERR tap-id: dump failed"; return 1; }
      local xy; xy="$(match_center "$rid" id)" || { echo "ERR tap-id: no node with id $rid"; return 1; }
      adb_ shell input tap $xy && echo "OK tap-id $rid @ $xy" || { echo "ERR tap-id failed"; return 1; };;
    text)
      local s="$*"; local esc="${s// /%s}"
      adb_ shell input text "$esc" && echo "OK text ${s@Q}" || { echo "ERR text failed"; return 1; };;
    key)
      local kc; kc="$(keycode "$1")"
      adb_ shell input keyevent "$kc" && echo "OK key $1" || { echo "ERR key failed"; return 1; };;
    swipe)
      [ $# -ge 4 ] || { echo "ERR swipe: need <x1> <y1> <x2> <y2> [ms]"; return 1; }
      adb_ shell input swipe "$1" "$2" "$3" "$4" "${5:-300}" && echo "OK swipe" || { echo "ERR swipe failed"; return 1; };;
    launch)
      local tgt="$1"
      if [[ "$tgt" == */* ]]; then
        adb_ shell am start -n "$tgt" >/dev/null 2>&1 && echo "OK launch $tgt" || { echo "ERR launch $tgt failed"; return 1; }
      else
        adb_ shell monkey -p "$tgt" -c android.intent.category.LAUNCHER 1 >/dev/null 2>&1 \
          && echo "OK launch $tgt" || { echo "ERR launch $tgt failed"; return 1; }
      fi;;
    stop)
      adb_ shell am force-stop "$1" && echo "OK stop $1" || { echo "ERR stop failed"; return 1; };;
    install)
      [ -f "$1" ] || { echo "ERR install: no file $1"; return 1; }
      adb_ install -r -g "$1" >/dev/null 2>&1 && echo "OK install $1" || { echo "ERR install failed"; return 1; };;
    state)
      local act; act="$(adb_ shell dumpsys activity activities 2>/dev/null | grep -m1 -E 'mResumedActivity|ResumedActivity' | tr -d '\r')"
      local pkg; pkg="$(adb_ shell dumpsys window 2>/dev/null | grep -m1 mCurrentFocus | tr -d '\r')"
      echo "focused: ${pkg:-?}"
      echo "resumed: ${act:-?}"
      echo "OK state";;
    wait)
      python3 -c "import time,sys; time.sleep(int(sys.argv[1])/1000)" "${1:-0}"; echo "OK wait ${1}ms";;
    shell)
      adb_ shell "$@" && echo "OK shell" || { echo "ERR shell failed"; return 1; };;
    *) echo "ERR unknown command: $cmd"; return 1;;
  esac
}

# Center-finder that injects the mode into the embedded python (text|id).
match_center() {
  local needle="$1" mode="$2"
  python3 - "$TMP/window.xml" "$needle" "$mode" <<'PY'
import sys, re, xml.etree.ElementTree as ET
path, needle, mode = sys.argv[1], sys.argv[2], sys.argv[3]
root = ET.parse(path).getroot()
def center(b):
    m=re.findall(r'\[(\d+),(\d+)\]', b or '')
    if len(m)!=2: return None
    (x1,y1),(x2,y2)=((int(a),int(c)) for a,c in m)
    return (x1+x2)//2,(y1+y2)//2
for n in root.iter():
    a=n.attrib
    if mode=="text":
        ok = needle in (a.get('text') or '') or needle in (a.get('content-desc') or '')
    else:
        rid=a.get('resource-id') or ''
        ok = rid==needle or rid.endswith('/'+needle) or rid.endswith(needle)
    if ok:
        c=center(a.get('bounds'))
        if c: print(c[0],c[1]); sys.exit(0)
sys.exit(3)
PY
}

# ---- device check ----------------------------------------------------------
if ! adb_ get-state >/dev/null 2>&1; then
  echo "ERR no device: start one with tools/android-agent-debug/boot-emulator.sh (or attach a device)" >&2
  exit 2
fi

# ---- read + run script -----------------------------------------------------
# Slurp the whole script first so adb commands can't consume the command stream.
SRC="${1:-/dev/stdin}"
mapfile -t LINES < "$SRC"

lineno=0
for line in "${LINES[@]}"; do
  lineno=$((lineno+1))
  line="${line#"${line%%[![:space:]]*}"}"   # ltrim
  [ -z "$line" ] && continue
  [ "${line:0:1}" = "#" ] && continue
  # Tokenize honoring double quotes.
  eval "set -- $line" 2>/dev/null || { echo "ERR line $lineno: parse error"; fail=1; continue; }
  run_cmd "$@" || fail=1
done

exit $fail
