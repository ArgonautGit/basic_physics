# physics

A small egui/eframe desktop app wrapping an in-progress 2D physics engine.

```sh
cargo run        # launch the GUI (needs a display)
cargo test       # run the headless UI + logic tests
```

## Agentic debugging (headless)

The app can be driven and inspected **without a display**, so an AI agent (or CI)
can see and interact with the UI to debug and test it. This uses
[`egui_kittest`](https://crates.io/crates/egui_kittest), egui's official headless
test harness: it steps the UI with no window, drives widgets through the
accessibility tree, and renders frames to PNG via wgpu.

Two independent capabilities:

- **Interact + inspect** — click buttons, set sliders, type, dump the widget tree
  and app state. Needs **no GPU or display**.
- **See** — render the current frame to a PNG. Needs a wgpu adapter (software is
  fine, see [GPU setup](#gpu-setup-for-screenshots)).

### Debug driver

`examples/debug_driver.rs` runs the app headlessly and executes a small command
script (from a file argument or stdin). Every invocation starts from a fresh app,
so scripts are deterministic replays.

```sh
# From stdin:
printf 'tree\nclick "Click me"\nset Radius 30\nstate\n' \
  | cargo run --example debug_driver

# From a file:
cargo run --example debug_driver -- script.txt
```

Each command echoes an `OK ...` or `ERR ...` line; the process exits non-zero if
any command failed.

| Command | Description |
| --- | --- |
| `step <n>` | Advance `n` frames. |
| `settle` | Run until the app stops requesting repaints. |
| `click <label>` | Click the widget with that accessibility label. |
| `set <label> <value>` | Set a slider / drag-value to a numeric value. |
| `type <label> <text>` | Focus a text field and type into it. |
| `key <name>` | Press a key (`enter`, `tab`, `escape`, `space`, `up`/`down`/`left`/`right`, `a`–`z`, `0`–`9`). |
| `tree` | Dump the accessibility tree (the agent's "inspect element"). |
| `state` | Debug-print the app state (`MyApp`). |
| `screenshot <path>` | Render the current frame to a PNG (needs a GPU adapter). |

Labels are matched exactly first, then by substring. Quote labels that contain
spaces (`click "Click me"`). Run `tree` to see the available labels, e.g.:

```
Window
  Button label="Click me"
  Label value="Clicks: 0"
  Slider label="Radius" numeric=100
  TextInput label="Name:" value=""
```

A typical agent loop: `tree` to discover widgets → `screenshot` to see the UI →
`click` / `set` / `type` to interact → `state` / `screenshot` to observe the
effect → read the PNG and printed output.

### Tests

- `tests/ui_interaction.rs` — headless interaction tests (no GPU). Run with plain
  `cargo test`.
- `tests/ui_snapshots.rs` — screenshot/snapshot tests (need a GPU adapter). Marked
  `#[ignore]` so `cargo test` stays green without one. Baselines live in
  `tests/snapshots/*.png`.

```sh
source scripts/headless-gpu.sh
UPDATE_SNAPSHOTS=1 cargo test --test ui_snapshots -- --ignored   # (re)write baselines
cargo test --test ui_snapshots -- --ignored                      # compare vs baselines
```

### GPU setup for screenshots

Screenshots and snapshot tests need a wgpu adapter; software rendering works.
`scripts/headless-gpu.sh` picks one automatically and exports the right env vars:

```sh
source scripts/headless-gpu.sh                 # export into your shell, then run cargo
scripts/headless-gpu.sh cargo test --test ui_snapshots -- --ignored   # or wrap a command
```

Its fallback chain: SwiftShader Vulkan ICD (bundled with the pre-installed
Chromium) → lavapipe Mesa ICD → wgpu's software GL backend. If none is present,
install Mesa's software Vulkan driver:

```sh
apt-get install -y mesa-vulkan-drivers libvulkan1
```

Interaction, `tree`, and `state` never need a GPU — only screenshots do.
