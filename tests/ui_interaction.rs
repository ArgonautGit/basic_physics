//! Headless interaction tests for the egui UI. These drive widgets through the
//! accessibility tree and need no GPU/display, so they run anywhere `cargo test` does.

use egui::accesskit::{Action, ActionData, ActionRequest};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use physics::ui::MyApp;

fn harness<'a>() -> Harness<'a, MyApp> {
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(400.0, 400.0))
        .with_step_dt(1.0 / 60.0)
        .with_max_steps(10_000)
        .build_eframe(|cc| MyApp::new(cc));
    harness.run_ok();
    harness
}

#[test]
fn initial_widgets_exist() {
    let harness = harness();
    // Panics if the widget is missing, which is the assertion we want.
    harness.get_by_label("Click me");
    harness.get_all_by_label("Radius").next().expect("Radius slider present");
    harness.get_by_label_contains("Physics Simulator");
    assert_eq!(harness.state().clicks, 0);
}

#[test]
fn clicking_button_increments_counter() {
    let mut harness = harness();
    harness.get_by_label("Click me").click();
    harness.step();
    harness.get_by_label("Click me").click();
    harness.step();

    assert_eq!(harness.state().clicks, 2);
    // The readout label reflects the new state.
    harness.get_by_label_contains("Clicks: 2");
}

#[test]
fn slider_set_value_updates_state() {
    let mut harness = harness();
    let target = harness.get_all_by_label("Radius").next().unwrap().id();
    harness.input_mut().events.push(egui::Event::AccessKitActionRequest(ActionRequest {
        action: Action::SetValue,
        target,
        data: Some(ActionData::NumericValue(30.0)),
    }));
    harness.step();

    assert_eq!(harness.state().radius, 30.0);
    // The slider node reports the new numeric value through accesskit.
    let numeric = harness.get_all_by_label("Radius").next().unwrap().numeric_value();
    assert_eq!(numeric, Some(30.0));
}

#[test]
fn typing_into_named_field_updates_state() {
    let mut harness = harness();
    let field = harness.get_by_label("Name:");
    field.focus();
    field.type_text("Ada");
    harness.step();

    assert_eq!(harness.state().name, "Ada");
    harness.get_by_label_contains("Hello, Ada!");
}
