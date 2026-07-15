# Debugging Android ARM apps headlessly (agent guide)

You can **see** and **interact with** an Android app in this cloud container with
no display or physical device. It's the Android analog of the egui headless
driver: a software-emulated **ARM64** Android runs under adb, and a driver script
turns adb into agent-friendly commands.

- **See** — `screenshot` (a PNG you can read), `tree` (the view hierarchy).
- **Interact** — `tap` / `tap-text` / `tap-id`, `text`, `key`, `swipe`, `launch`.

Unlike the egui driver (fresh replay each run), this drives the **live** device, so
state persists between invocations.

## Running ARM apps here (and why it's slow)

This container is x86_64 with **no `/dev/kvm`** — no nested virtualization. Two
things follow, and together they pin down the one config that works:

- The **current Android Emulator requires the system image arch to match the
  host.** An `arm64-v8a` image is rejected on an x86_64 host
  (`Avd's CPU Architecture 'arm64' is not supported ... on x86_64 host`), and the
  legacy emulators that did ARM-on-x86 translation are no longer downloadable.
- An **x86_64** image *does* run — its arch matches the host — and boots in pure
  software with `-accel off` (no KVM). Google's x86_64 system images (API 30+)
  ship **arm64-v8a binary translation**, so **ARM apps still run** on them. That's
  how ARM APKs execute on this x86_64 host.

The cost of software (TCG) CPU + software GL: first boot takes several minutes and
interactions have noticeable latency. Be patient; it works.

## One-time setup

```sh
tools/android-agent-debug/setup-emulator.sh      # SDK + arm64 image + AVD (~5 GB, slow)
tools/android-agent-debug/boot-emulator.sh       # boot headless, wait for ready (slow)
```

`setup-emulator.sh` installs the SDK under `~/android-sdk`, picks an available
`x86_64` system image (which carries the ARM translation layer), creates an AVD
named `agent-x86_64`, and caps its userdata partition to fit the disk allowance.
`boot-emulator.sh` launches it with `-no-window -accel off -gpu
swiftshader_indirect` and blocks until `sys.boot_completed=1`. Both are ephemeral
to the session — re-run in a fresh container.

Sanity check once booted:

```sh
export PATH="$HOME/android-sdk/platform-tools:$PATH"
adb devices        # should list an 'emulator-5554  device' line
```

## Driving the app

```sh
# From stdin:
printf 'launch com.android.settings\ntree\nscreenshot shots/s.png\n' \
  | tools/android-agent-debug/adb-driver.sh

# From a file:
tools/android-agent-debug/adb-driver.sh script.txt
```

Each command prints `OK ...` or `ERR ...`; the process exits non-zero if any
failed. `#` comments and blank lines are ignored; quote args with spaces.

| Command | What it does |
| --- | --- |
| `screenshot <path>` | Save a PNG of the current screen. |
| `tree` | Print a compact view hierarchy (class, text, id, bounds). **Start here** to find targets. |
| `tap <x> <y>` | Tap raw screen coordinates. |
| `tap-text <substring>` | Tap the first node whose text/content-desc contains this. |
| `tap-id <resource-id>` | Tap the node with this resource-id (full id or a suffix). |
| `text <string>` | Type into the focused field. |
| `key <name>` | Key event: `enter back home tab del space up down left right menu app_switch` (or a raw keycode). |
| `swipe <x1> <y1> <x2> <y2> [ms]` | Swipe gesture (scroll, dismiss). |
| `launch <pkg>[/<activity>]` | Start an app (uses monkey if no activity). |
| `stop <pkg>` | Force-stop an app. |
| `install <apk>` | Install / reinstall an APK (granting runtime perms). |
| `state` | Print the current focused window + resumed activity. |
| `wait <ms>` | Sleep (let a transition/animation settle). |
| `shell <cmd...>` | Arbitrary `adb shell` escape hatch. |

### Reading the tree

`tree` prints one row per node that carries text, a content-desc, a resource-id,
is clickable, or is a leaf — with its `[left,top][right,bottom]` bounds:

```
FrameLayout id='com.android.settings:id/main_content' [0,0][1080,2148]
  TextView text='Settings' [42,132][300,210]
  RecyclerView id='.../recycler_view' scrollable [0,210][1080,2148]
    LinearLayout text='Network & internet' clickable [0,210][1080,420]
    LinearLayout text='Connected devices' clickable [0,420][1080,630]
```

Then target by text or id instead of raw coordinates:

```
tap-text "Network & internet"
```

`tap-text` / `tap-id` compute the node's center from its bounds, so you rarely need
literal `tap <x> <y>` — use that only when nothing carries a label.

## The debugging loop

1. `launch <your.package>` (or `install app.apk` then `launch`).
2. `tree` — see what's on screen and the targetable labels/ids.
3. `screenshot before.png` — look at it.
4. `tap-text` / `text` / `key` / `swipe` — drive the flow.
5. `wait 500` for the transition, then `tree` / `screenshot` / `state` — observe.
6. Read the PNG and printed output; decide the next step.

Example: exercise your app's login screen.

```
install build/outputs/apk/debug/app-debug.apk
launch com.example.myapp
wait 800
tree
tap-id username
text alice@example.com
tap-id password
text hunter2
tap-text "Sign in"
wait 1000
screenshot shots/after-login.png
state
```

## Targeting your own APK

The toolkit is app-agnostic. Point it at any ARM-compatible APK:

```sh
tools/android-agent-debug/adb-driver.sh <<'EOF'
install path/to/app-debug.apk
launch com.your.package
tree
EOF
```

ARM (`arm64-v8a`) and x86_64 native libs both run: the x86_64 image executes
x86_64 code directly and translates arm64-v8a. A 32-bit-only `armeabi-v7a` APK may
not — prefer an `arm64-v8a`, `x86_64`, or universal build.

## Performance & stability (software emulation is fragile)

Validated on this container end-to-end: the emulator boots, `screenshot` captures
the real UI, `tree` returns the parsed hierarchy, and `tap`/`tap-text`/`swipe`/
`key` drive it. But with no KVM everything runs on QEMU's software CPU + software
GL, so expect roughness and plan around it:

- **Cold boot takes 10–20 min** and pegs a CPU core the whole time. `sys.boot_completed`
  flips late — after the package manager is already up. Be patient.
- **SystemUI throws ANRs** ("System UI isn't responding") under load. Dismiss with
  `tap` on *Wait*, or just `wait` and re-`screenshot`.
- **`uiautomator dump` is heavy** — it can take 30–80 s and, under memory pressure
  during/just after boot, get OOM-killed. The driver bounds it with `DUMP_TIMEOUT`
  (default 45 s) and returns `ERR` instead of hanging. If `tree`/`tap-text` fails,
  `wait 3000`, let the screen settle, and retry — or fall back to `screenshot` +
  `tap <x> <y>` (coordinate taps never need a dump).
- **The screen can briefly go black** (a ~15 KB screenshot) between transitions
  while a surface is being drawn. Re-`screenshot` after a `wait`.
- **Give generous `wait`s** between an interaction and the observation — navigation
  that's instant on hardware can take several seconds here.
- **Prefer `-no-snapshot` cold boots** (the scripts do). Snapshots are less reliable
  in this mode.

None of these are toolkit bugs — they're the cost of running Android without
hardware virtualization. On a host *with* `/dev/kvm`, switch the image to `x86_64`
with acceleration (drop `-accel off` in `boot-emulator.sh`) and it's fast and
stable; the driver commands are identical.

## Gotchas

- **Boot is slow.** `boot-emulator.sh` waits up to `BOOT_TIMEOUT` (default 900 s).
  If it times out, check `~/android-sdk/emulator.log`.
- **`uiautomator dump` can fail** mid-animation or on secure/FLAG_SECURE screens.
  `wait 500` and retry `tree`; if it still fails, fall back to `screenshot` +
  `tap <x> <y>`.
- **One emulator at a time** is assumed. For several devices set `ANDROID_SERIAL`
  (the adb serial) before calling the driver.
- **PATH** — the scripts add `~/android-sdk/platform-tools` and `.../emulator`
  themselves; add them to your own shell if you call `adb`/`emulator` directly.
