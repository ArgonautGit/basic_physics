# Driving the egui app headlessly (agent guide)

You can **see** and **interact with** this app's egui UI without a display. Use it
to reproduce bugs, verify a change, or inspect widget state. Two capabilities:

- **Interact + inspect** — click, set sliders, type, dump the widget tree and app
  state. Needs **no GPU or display**.
- **See** — render the current frame to a PNG you can open/read. Needs a software
  GPU adapter (one-time env setup, below).

The engine is `examples/debug_driver.rs`, built on `egui_kittest`. It steps the app
with no window, drives widgets through the accessibility tree, and renders frames
via wgpu. **Every run starts from a fresh app**, so a script is a deterministic
replay — put the whole scenario in one script rather than expecting state to
persist between invocations.

## Run it

```sh
# Pipe a script on stdin:
printf 'tree\nclick "Click me"\nset Radius 30\nstate\n' | cargo run --example debug_driver

# Or from a file:
cargo run --example debug_driver -- script.txt
```

Each command prints `OK ...` or `ERR ...`. The process exits non-zero if any
command failed. Lines starting with `#` and blank lines are ignored.

## Commands

| Command | What it does |
| --- | --- |
| `tree` | Dump the accessibility tree — every widget with its role, label, value. **Start here** to learn the labels. |
| `state` | Debug-print the app state (`MyApp`: `clicks`, `radius`, `name`). |
| `click <label>` | Click the widget with that label. |
| `set <label> <value>` | Set a slider / numeric field to a value. |
| `type <label> <text>` | Focus a text field and type into it. |
| `key <name>` | Press a key: `enter`, `tab`, `escape`, `space`, `up`/`down`/`left`/`right`, `a`–`z`, `0`–`9`. |
| `step <n>` | Advance `n` frames. |
| `settle` | Run until the app stops requesting repaints. |
| `screenshot <path>` | Render the current frame to a PNG (needs a GPU adapter — see below). |

### Labels

- Matched **exactly first, then by substring**. `set Radius 30` and `set "Radius" 30`
  both work; substring means `type Name Ada` finds the field labelled `Name:`.
- **Quote labels containing spaces:** `click "Click me"`.
- If several widgets share a label the driver takes the first (e.g. a slider
  exposes both a Slider and a SpinButton node — either accepts `set`).
- Don't guess labels — run `tree` and read them off. Example output:

```
Window
  Label value="Physics Simulator"
  Button label="Click me"
  Label value="Clicks: 0"
  Slider label="Radius" numeric=100
  SpinButton label="Radius" value="100" numeric=100
  Label value="Radius"
  TextInput label="Name:" value=""
  Label value="Hello, world!"
```

## The debugging loop

1. `tree` — discover the widgets and their current values.
2. `screenshot before.png` — see the starting state (optional; needs GPU).
3. `click` / `set` / `type` — perform the interaction you want to test.
4. `state` and/or `screenshot after.png` — observe the effect.
5. Read the printed output and the PNGs, then decide the next step.

Full example script:

```
# reproduce: does shrinking the radius + clicking update everything?
tree
screenshot shots/before.png
set Radius 25
click "Click me"
type Name Grace
state
screenshot shots/after.png
```

Expected tail of the output:

```
OK set "Radius" = 25
OK click "Click me"
OK type "Grace" into "Name"
MyApp {
    clicks: 1,
    radius: 25.0,
    name: "Grace",
}
OK state
OK screenshot shots/after.png (400x400)
```

## Screenshots: GPU setup (one time per shell)

Screenshots (and the snapshot tests) need a wgpu adapter; software rendering is
fine. `scripts/headless-gpu.sh` finds one and exports the right env vars:

```sh
source scripts/headless-gpu.sh          # export into your shell, then run cargo
# or wrap a single command:
scripts/headless-gpu.sh cargo run --example debug_driver -- script.txt
```

Fallback chain: SwiftShader Vulkan ICD (bundled with the pre-installed Chromium)
→ lavapipe Mesa ICD → software GL. If none exists, install one:

```sh
apt-get install -y mesa-vulkan-drivers libvulkan1
```

**Gotcha:** `VAR=x ... | cargo run` sets `VAR` only for the first command in the
pipeline, not for `cargo`. Either `source scripts/headless-gpu.sh` first, or
`export` the vars, so they actually reach cargo.

Without a GPU, `screenshot` prints `ERR` and the run continues — `tree`, `state`,
and all interaction still work.

## Tests

- `cargo test` — headless interaction tests (`tests/ui_interaction.rs`), no GPU.
- Snapshot tests (`tests/ui_snapshots.rs`) are `#[ignore]`d so `cargo test` stays
  green without a GPU. To run them:

  ```sh
  source scripts/headless-gpu.sh
  UPDATE_SNAPSHOTS=1 cargo test --test ui_snapshots -- --ignored   # (re)write baselines
  cargo test --test ui_snapshots -- --ignored                      # compare
  ```

  Baselines live in `tests/snapshots/*.png`. On mismatch, kittest writes
  `*.new.png` / `*.diff.png` next to them (gitignored) for inspection.

## Adding widgets to the app

New widgets show up automatically in `tree` and are drivable — with two rules for
accessibility:

- Give interactive widgets a **label** (`Slider::new(...).text("Radius")`,
  `Button::new("Click me")`).
- A bare `text_edit_singleline` has no label. Associate a preceding label so the
  agent can target it:

  ```rust
  let label = ui.label("Name:");
  ui.text_edit_singleline(&mut self.name).labelled_by(label.id);
  ```
