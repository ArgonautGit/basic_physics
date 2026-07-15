//! Snapshot (screenshot) tests for the egui UI. These render real frames via wgpu,
//! so they need a software/hardware GPU adapter (see `scripts/headless-gpu.sh`) and
//! are marked `#[ignore]` so a plain `cargo test` on a machine with no adapter stays
//! green. Run them explicitly:
//!
//!   source scripts/headless-gpu.sh
//!   UPDATE_SNAPSHOTS=1 cargo test --test ui_snapshots -- --ignored   # write baselines
//!   cargo test --test ui_snapshots -- --ignored                      # compare
//!
//! Baselines live in tests/snapshots/*.png and are committed; on mismatch kittest
//! writes *.new.png / *.diff.png next to them (gitignored).

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use physics::ui::MyApp;

fn harness<'a>() -> Harness<'a, MyApp> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 400.0))
        .with_step_dt(1.0 / 60.0)
        .with_max_steps(10_000)
        .wgpu()
        .build_eframe(|cc| MyApp::new(cc));
    harness.run_ok();
    harness
}

#[test]
#[ignore = "needs a wgpu adapter; see scripts/headless-gpu.sh"]
fn initial_ui() {
    let mut harness = harness();
    harness.snapshot("initial_ui");
}

#[test]
#[ignore = "needs a wgpu adapter; see scripts/headless-gpu.sh"]
fn after_interaction() {
    let mut harness = harness();
    // Shrink the circle and click the button so the frame visibly differs from initial.
    harness.get_by_label("Click me").click();
    harness.step();
    let target = harness.get_all_by_label("Radius").next().unwrap().id();
    harness.input_mut().events.push(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target,
            data: Some(egui::accesskit::ActionData::NumericValue(40.0)),
        },
    ));
    harness.step();
    harness.snapshot("after_interaction");
}
