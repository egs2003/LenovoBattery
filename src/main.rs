#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod energy;

use eframe::egui::{self, Color32, RichText};
use energy::{Driver, Mode};

struct App {
    status: Result<Mode, String>,
}

impl App {
    fn new() -> Self {
        let mut app = App { status: Err(String::new()) };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.status = Driver::open().and_then(|d| d.mode());
    }

    fn set(&mut self, target: Mode) {
        self.status = Driver::open().and_then(|d| d.set_mode(target));
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Legion Battery Mode");
            ui.add_space(8.0);

            match &self.status {
                Ok(mode) => ui.label(
                    RichText::new(format!("Aktueller Modus: {}", mode.label())).color(Color32::LIGHT_BLUE),
                ),
                Err(e) => ui.label(RichText::new(format!("FEHLER: {e}")).color(Color32::LIGHT_RED)),
            };
            ui.add_space(12.0);

            let current = self.status.as_ref().ok().copied();
            let mut target = None;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(current != Some(Mode::Normal), egui::Button::new("Normal"))
                    .clicked()
                {
                    target = Some(Mode::Normal);
                }
                if ui
                    .add_enabled(current != Some(Mode::Storage), egui::Button::new("Storage (max. 80%)"))
                    .clicked()
                {
                    target = Some(Mode::Storage);
                }
                if ui.button("Aktualisieren").clicked() {
                    self.refresh();
                }
            });
            if let Some(t) = target {
                self.set(t);
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 140.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Legion Battery Mode",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
