#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod drawing;

use eframe::{egui, Frame};
use egui::{Button, Color32, Pos2};

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((300.0, 300.0)),
        ..Default::default()
    };

    eframe::run_native("Test App", options, Box::new(|_cc| Ok(Box::<MyApp>::default())))
}

#[derive(Default)]
struct MyApp {
    // Put state here.
}

impl eframe::App for MyApp {
    // Runs every frame.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
         egui::CentralPanel::default().show(ctx, |ui| {
            

            ui.heading("Physics Simulator");

            let painter = ui.painter();
            painter.circle_filled(Pos2::new(150.0, 150.0), 100.0, egui::Color32::DARK_BLUE);
        });

        ctx.request_repaint();
    }
}