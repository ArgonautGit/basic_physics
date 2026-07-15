mod drawing;

use eframe::egui;
use egui::{Color32, Pos2, Slider};

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((400.0, 400.0)),
        ..Default::default()
    };

    eframe::run_native("Test App", options, Box::new(|cc| Ok(Box::new(MyApp::new(cc)))))
}

#[derive(Debug)]
pub struct MyApp {
    pub clicks: u32,
    pub radius: f32,
    pub name: String,
}

impl Default for MyApp {
    fn default() -> Self {
        MyApp { clicks: 0, radius: 100.0, name: String::new() }
    }
}

impl MyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for MyApp {
    // Runs every frame.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Physics Simulator");

            ui.horizontal(|ui| {
                if ui.button("Click me").clicked() {
                    self.clicks += 1;
                }
                ui.label(format!("Clicks: {}", self.clicks));
            });

            ui.add(Slider::new(&mut self.radius, 10.0..=140.0).text("Radius"));

            ui.horizontal(|ui| {
                let label = ui.label("Name:");
                // Associate the label so the text field is reachable by label
                // from the accessibility tree (e.g. the debug driver's `type Name`).
                ui.text_edit_singleline(&mut self.name).labelled_by(label.id);
            });
            ui.label(format!("Hello, {}!", if self.name.is_empty() { "world" } else { &self.name }));

            let painter = ui.painter();
            painter.circle_filled(Pos2::new(200.0, 250.0), self.radius, Color32::DARK_BLUE);
        });
    }
}
