#!/usr/bin/env bash
# Boot the AVD headless and wait until it is fully booted and ready for adb.
# Software (TCG) boot is slow — allow several minutes. Leaves the emulator running.
set -euo pipefail

export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/android-sdk}"
export ANDROID_HOME="$ANDROID_SDK_ROOT"
export PATH="$ANDROID_SDK_ROOT/platform-tools:$ANDROID_SDK_ROOT/emulator:$ANDROID_SDK_ROOT/cmdline-tools/latest/bin:$PATH"
AVD_NAME="${AVD_NAME:-agent-x86_64}"
BOOT_TIMEOUT="${BOOT_TIMEOUT:-900}"   # seconds

log() { echo "[boot] $*" >&2; }

# Start a fresh adb server, then launch the emulator headless in the background.
adb start-server >/dev/null 2>&1 || true

if adb devices | grep -q emulator; then
  log "an emulator is already running; reusing it"
else
  log "launching '$AVD_NAME' headless (software mode — slow, be patient)"
  # -no-window: no display. -accel off: pure software CPU (no KVM on this host).
  # -gpu swiftshader_indirect: software GL. -no-snapshot: clean boot.
  # -partition-size 4096: match the capped userdata partition.
  nohup emulator -avd "$AVD_NAME" \
    -no-window -no-audio -no-boot-anim -no-snapshot \
    -accel off -gpu swiftshader_indirect -partition-size 4096 \
    -netdelay none -netspeed full \
    > "$ANDROID_SDK_ROOT/emulator.log" 2>&1 &
  log "emulator pid $! (log: $ANDROID_SDK_ROOT/emulator.log)"
fi

log "waiting for device to appear on adb"
adb wait-for-device

log "waiting for sys.boot_completed (timeout ${BOOT_TIMEOUT}s)"
deadline=$(( $(cut -d. -f1 /proc/uptime) + BOOT_TIMEOUT ))
while true; do
  booted="$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')"
  [ "$booted" = "1" ] && break
  if [ "$(cut -d. -f1 /proc/uptime)" -ge "$deadline" ]; then
    log "ERROR: boot did not complete within ${BOOT_TIMEOUT}s (see emulator.log)"
    exit 1
  fi
  sleep 5
done

# Dismiss the lock screen and settle.
adb shell input keyevent 82 >/dev/null 2>&1 || true
log "device booted and ready:"
adb devices | grep emulator >&2
