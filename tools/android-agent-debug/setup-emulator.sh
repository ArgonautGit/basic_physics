#!/usr/bin/env bash
# Install the Android SDK + a system image and create a headless AVD.
#
# GPU/CPU reality in this cloud container: x86_64 host with NO /dev/kvm (no nested
# virtualization). The modern Android Emulator REQUIRES the system image arch to
# match the host, so an arm64-v8a image is rejected outright on an x86_64 host
# ("Avd's CPU Architecture 'arm64' is not supported ... on x86_64 host"). The
# config that actually runs here is an **x86_64** image booted with `-accel off`
# (pure software/TCG, no KVM needed). Crucially, Google's x86_64 images API 30+
# ship ARM (arm64-v8a) binary translation, so ARM apps still run on them.
#
# Usage:
#   tools/android-agent-debug/setup-emulator.sh            # install + create AVD
#   tools/android-agent-debug/setup-emulator.sh --boot     # also boot it headless
#
# Everything installs under $ANDROID_SDK_ROOT (default ~/android-sdk). This is
# ephemeral to the session; re-run in a fresh container.
set -euo pipefail

export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/android-sdk}"
export ANDROID_HOME="$ANDROID_SDK_ROOT"
CMDLINE_VER="11076708"   # commandlinetools-linux build; latest as of writing
AVD_NAME="${AVD_NAME:-agent-x86_64}"

# Candidate system images, most-preferred first. x86_64 (with built-in arm64
# translation) — arm64 images do NOT run on an x86_64 host with the current
# emulator, so we do not offer them.
IMAGES=(
  "system-images;android-33;google_apis;x86_64"
  "system-images;android-34;google_apis;x86_64"
  "system-images;android-31;google_apis;x86_64"
  "system-images;android-30;google_apis;x86_64"
)

log() { echo "[setup] $*" >&2; }

install_cmdline_tools() {
  local dest="$ANDROID_SDK_ROOT/cmdline-tools/latest"
  if [ -x "$dest/bin/sdkmanager" ]; then log "cmdline-tools present"; return; fi
  log "downloading command-line tools"
  mkdir -p "$ANDROID_SDK_ROOT/cmdline-tools"
  local zip="/tmp/cmdline-tools.zip"
  curl -fsSL -o "$zip" \
    "https://dl.google.com/android/repository/commandlinetools-linux-${CMDLINE_VER}_latest.zip"
  rm -rf "$ANDROID_SDK_ROOT/cmdline-tools/cmdline-tools"
  unzip -q "$zip" -d "$ANDROID_SDK_ROOT/cmdline-tools"
  mv "$ANDROID_SDK_ROOT/cmdline-tools/cmdline-tools" "$dest"
  rm -f "$zip"
}

sdkm() { "$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/sdkmanager" "$@"; }

main() {
  command -v java >/dev/null || { log "java not found"; exit 1; }
  install_cmdline_tools
  export PATH="$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$ANDROID_SDK_ROOT/platform-tools:$ANDROID_SDK_ROOT/emulator:$PATH"

  log "accepting licenses"
  yes | sdkm --licenses >/dev/null 2>&1 || true

  log "installing platform-tools + emulator"
  sdkm --install "platform-tools" "emulator" >/dev/null

  local chosen=""
  for img in "${IMAGES[@]}"; do
    log "trying system image: $img"
    if sdkm --install "$img" >/dev/null 2>&1; then chosen="$img"; break; fi
  done
  [ -n "$chosen" ] || { log "no arm64-v8a system image could be installed"; exit 1; }
  log "installed image: $chosen"

  log "creating AVD '$AVD_NAME'"
  echo "no" | "$ANDROID_SDK_ROOT/cmdline-tools/latest/bin/avdmanager" create avd \
    --force --name "$AVD_NAME" --package "$chosen" --device "pixel_5" >/dev/null

  # Cap the userdata partition so the software-mode boot fits the disk allowance.
  local cfg="$HOME/.android/avd/$AVD_NAME.avd/config.ini"
  if [ -f "$cfg" ]; then
    if grep -q '^disk.dataPartition.size=' "$cfg"; then
      sed -i 's/^disk.dataPartition.size=.*/disk.dataPartition.size=4096M/' "$cfg"
    else
      echo 'disk.dataPartition.size=4096M' >> "$cfg"
    fi
  fi

  # Persist the image id so other scripts know what was set up.
  echo "$chosen" > "$ANDROID_SDK_ROOT/.agent-avd-image"
  log "AVD ready: $AVD_NAME  (image: $chosen)"

  if [ "${1:-}" = "--boot" ]; then
    log "booting headless (this is slow under software emulation)…"
    exec "$(dirname "$0")/boot-emulator.sh"
  fi
  log "done. Boot with: tools/android-agent-debug/boot-emulator.sh"
}

main "${1:-}"
