//! Agentic debug driver: run the app headlessly and script interactions against it.
//!
//! Usage:
//!   cargo run --example debug_driver -- script.txt
//!   printf 'tree\nclick "Click me"\nstate\n' | cargo run --example debug_driver
//!
//! Commands (one per line, `#` starts a comment; labels with spaces need quotes):
//!   step <n>              advance n frames
//!   settle                run until the app stops requesting repaints
//!   click <label>         click the widget with that accessibility label
//!   set <label> <value>   set a slider/drag-value to a numeric value
//!   type <label> <text>   focus a text field and type into it
//!   key <name>            press a key (enter, tab, escape, space, arrow keys, a-z, 0-9)
//!   tree                  dump the accessibility tree (the agent's "inspect DOM")
//!   state                 debug-print the app state (MyApp)
//!   screenshot <path>     render the current frame to a PNG (needs a wgpu adapter;
//!                         see scripts/headless-gpu.sh)
//!
//! Each command echoes an `OK ...` or `ERR ...` line; exit code is 1 if any command failed.
//! Every invocation starts from a fresh app, so scripts are deterministic replays.

use std::io::Read as _;
use std::panic::{AssertUnwindSafe, catch_unwind};

use egui::accesskit::{Action, ActionData, ActionRequest};
use egui_kittest::kittest::{Node, Queryable};
use egui_kittest::Harness;
use physics::ui::MyApp;

fn main() {
    let script = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(&path)
            .unwrap_or_else(|e| exit_err(&format!("cannot read script {path}: {e}"))),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .unwrap_or_else(|e| exit_err(&format!("cannot read stdin: {e}")));
            buf
        }
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 400.0))
        .with_step_dt(1.0 / 60.0)
        .with_max_steps(10_000)
        .build_eframe(|cc| MyApp::new(cc));
    harness.step();

    let mut failed = false;
    for (lineno, line) in script.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match run_command(&mut harness, line) {
            Ok(msg) => println!("OK {msg}"),
            Err(msg) => {
                println!("ERR line {}: {msg}", lineno + 1);
                failed = true;
            }
        }
    }

    std::process::exit(if failed { 1 } else { 0 });
}

fn exit_err(msg: &str) -> ! {
    eprintln!("ERR {msg}");
    std::process::exit(1)
}

fn run_command(harness: &mut Harness<'_, MyApp>, line: &str) -> Result<String, String> {
    let tokens = tokenize(line)?;
    let (cmd, args) = tokens.split_first().ok_or("empty command")?;

    match (cmd.as_str(), args) {
        ("step", [n]) => {
            let n: u64 = n.parse().map_err(|_| format!("bad step count {n:?}"))?;
            harness.run_steps(n as usize);
            Ok(format!("step {n}"))
        }
        ("settle", []) => match harness.run_ok() {
            Some(frames) => Ok(format!("settle ({frames} frames)")),
            None => Ok("settle (did not stabilize within max_steps)".into()),
        },
        ("click", [label]) => {
            let node = find_node(harness, label)?;
            node.click();
            drop(node);
            harness.step();
            Ok(format!("click {label:?}"))
        }
        ("set", [label, value]) => {
            let value: f64 = value.parse().map_err(|_| format!("bad numeric value {value:?}"))?;
            let target = find_node(harness, label)?.id();
            harness.input_mut().events.push(egui::Event::AccessKitActionRequest(ActionRequest {
                action: Action::SetValue,
                target,
                data: Some(ActionData::NumericValue(value)),
            }));
            harness.step();
            Ok(format!("set {label:?} = {value}"))
        }
        ("type", [label, text]) => {
            let node = find_node(harness, label)?;
            node.focus();
            node.type_text(text);
            drop(node);
            harness.step();
            Ok(format!("type {text:?} into {label:?}"))
        }
        ("key", [name]) => {
            let key = parse_key(name)?;
            harness.press_key(key);
            harness.step();
            Ok(format!("key {name}"))
        }
        ("tree", []) => {
            let mut out = String::new();
            // kittest's Node derefs to an accesskit_consumer node; walk that.
            dump_node(&harness.node(), 0, &mut out);
            print!("{out}");
            Ok("tree".into())
        }
        ("state", []) => {
            println!("{:#?}", harness.state());
            Ok("state".into())
        }
        ("screenshot", [path]) => {
            let image = catch_unwind(AssertUnwindSafe(|| harness.render()))
                .map_err(|_| "renderer panicked (no wgpu adapter? see scripts/headless-gpu.sh)")?
                .map_err(|e| format!("render failed: {e}"))?;
            if let Some(dir) = std::path::Path::new(path).parent() {
                if !dir.as_os_str().is_empty() {
                    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {dir:?}: {e}"))?;
                }
            }
            image.save(path).map_err(|e| format!("save {path:?}: {e}"))?;
            Ok(format!("screenshot {path} ({}x{})", image.width(), image.height()))
        }
        _ => Err(format!(
            "unknown command or wrong arg count: {line:?} (commands: step settle click set type key tree state screenshot)"
        )),
    }
}

/// Find a node by exact accessibility label, falling back to substring match.
/// The label and the harness borrow share a lifetime because kittest's query
/// filter (`By<'a>`) ties the matched node's lifetime to the label's. We take the
/// first match (via the `_all_` variants) rather than `query_by_label`, which
/// panics when several widgets share a label (e.g. a Slider renders both a
/// Slider and a SpinButton node with the same label).
fn find_node<'a>(harness: &'a Harness<'_, MyApp>, label: &'a str) -> Result<Node<'a>, String> {
    harness
        .query_all_by_label(label)
        .next()
        .or_else(|| harness.query_all_by_label_contains(label).next())
        .ok_or_else(|| format!("no widget with label {label:?} (try `tree`)"))
}

/// Recursively render the accessibility tree. `node` is an accesskit_consumer node
/// (kittest's `Node` derefs into one, so the root passes through transparently).
fn dump_node(node: &accesskit_consumer::Node<'_>, depth: usize, out: &mut String) {
    let mut line = format!("{}{:?}", "  ".repeat(depth), node.role());
    if let Some(label) = node.label() {
        line.push_str(&format!(" label={label:?}"));
    }
    if let Some(value) = node.value() {
        line.push_str(&format!(" value={value:?}"));
    }
    if let Some(numeric) = node.numeric_value() {
        line.push_str(&format!(" numeric={numeric}"));
    }
    if let Some(toggled) = node.toggled() {
        line.push_str(&format!(" toggled={toggled:?}"));
    }
    if node.is_focused() {
        line.push_str(" focused");
    }
    out.push_str(&line);
    out.push('\n');
    for child in node.children() {
        dump_node(&child, depth + 1, out);
    }
}

fn parse_key(name: &str) -> Result<egui::Key, String> {
    egui::Key::from_name(&normalize_key_name(name))
        .ok_or_else(|| format!("unknown key {name:?}"))
}

fn normalize_key_name(name: &str) -> String {
    // egui::Key::from_name expects names like "Enter", "A", "ArrowDown".
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "up" => "ArrowUp".into(),
        "down" => "ArrowDown".into(),
        "left" => "ArrowLeft".into(),
        "right" => "ArrowRight".into(),
        _ => {
            let mut chars = lower.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

/// Split a command line into tokens, honoring double quotes. The last argument of
/// `type` may contain spaces if quoted.
fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => in_quotes = !in_quotes,
            '\\' if in_quotes && chars.peek() == Some(&'"') => {
                current.push(chars.next().unwrap());
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if in_quotes {
        return Err("unterminated quote".into());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}
