#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod energy;
mod update;

use eframe::egui::{self, Color32, RichText};
use energy::{Driver, Mode};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use update::UpState;

const APP_TITLE: &str = "Legion Battery Mode";

// Farben: je nach Systemdesign (dunkel/hell). `pal()` liefert die Palette des aktuellen Themes,
// `update` setzt sie zu Beginn jedes Frames anhand von `ctx.theme()`.
#[derive(Clone, Copy)]
struct Pal {
    bg: Color32,
    card: Color32,
    border: Color32,
    widget: Color32,
    widget_hover: Color32,
    accent: Color32,
    /// Hintergrund fuer ausgewaehlte Elemente (Segmente, Textmarkierung)
    sel_bg: Color32,
    red: Color32,
    green: Color32,
    amber: Color32,
    muted: Color32,
    info_bg: Color32,
    info_text: Color32,
    err_bg: Color32,
    err_text: Color32,
    input_bg: Color32,
}

const DARK: Pal = Pal {
    bg: Color32::from_rgb(20, 22, 27),
    card: Color32::from_rgb(30, 33, 40),
    border: Color32::from_rgb(46, 50, 60),
    widget: Color32::from_rgb(41, 45, 55),
    widget_hover: Color32::from_rgb(52, 57, 70),
    accent: Color32::from_rgb(86, 132, 255),
    sel_bg: Color32::from_rgb(86, 132, 255),
    red: Color32::from_rgb(232, 64, 64),
    green: Color32::from_rgb(70, 190, 120),
    amber: Color32::from_rgb(235, 170, 50),
    muted: Color32::from_rgb(140, 147, 163),
    info_bg: Color32::from_rgb(26, 35, 64),
    info_text: Color32::from_rgb(169, 184, 230),
    err_bg: Color32::from_rgb(52, 26, 30),
    err_text: Color32::from_rgb(240, 170, 170),
    input_bg: Color32::from_rgb(24, 26, 32),
};

const LIGHT: Pal = Pal {
    bg: Color32::from_rgb(243, 244, 247),
    card: Color32::from_rgb(255, 255, 255),
    border: Color32::from_rgb(218, 222, 230),
    widget: Color32::from_rgb(234, 237, 243),
    widget_hover: Color32::from_rgb(222, 227, 237),
    accent: Color32::from_rgb(55, 100, 235),
    sel_bg: Color32::from_rgb(205, 220, 255),
    red: Color32::from_rgb(214, 48, 48),
    green: Color32::from_rgb(34, 158, 90),
    amber: Color32::from_rgb(214, 140, 20),
    muted: Color32::from_rgb(98, 106, 123),
    info_bg: Color32::from_rgb(228, 237, 255),
    info_text: Color32::from_rgb(45, 68, 130),
    err_bg: Color32::from_rgb(253, 232, 232),
    err_text: Color32::from_rgb(170, 35, 35),
    input_bg: Color32::from_rgb(250, 251, 253),
};

static IS_LIGHT: AtomicBool = AtomicBool::new(false);

fn pal() -> Pal {
    if IS_LIGHT.load(AtomicOrdering::Relaxed) {
        LIGHT
    } else {
        DARK
    }
}

/// Folgt der Systemeinstellung (dunkel/hell) und legt fuer beide Themes einen eigenen Style an.
fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::System);
    ctx.set_style_of(egui::Theme::Dark, build_style(&DARK, false));
    ctx.set_style_of(egui::Theme::Light, build_style(&LIGHT, true));
}

fn build_style(p: &Pal, light: bool) -> egui::Style {
    use egui::{FontFamily, FontId, Rounding, Stroke, TextStyle};
    let mut style = egui::Style::default();
    style.text_styles.insert(TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 26.0;

    let mut v = if light { egui::Visuals::light() } else { egui::Visuals::dark() };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.extreme_bg_color = p.input_bg;
    v.selection.bg_fill = p.sel_bg;
    // egui nimmt `selection.stroke` als Schriftfarbe ausgewaehlter Eintraege und `selection.bg_fill`
    // als deren Hintergrund. Beide duerfen nicht dieselbe Farbe haben.
    let sel_text = if light { Color32::from_rgb(25, 55, 150) } else { Color32::from_rgb(240, 244, 255) };
    v.selection.stroke = Stroke::new(1.0_f32, sel_text);
    v.hyperlink_color = p.accent;
    let r = Rounding::same(9.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    v.widgets.inactive.rounding = r;
    v.widgets.inactive.weak_bg_fill = p.widget;
    v.widgets.inactive.bg_fill = p.widget;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    v.widgets.hovered.rounding = r;
    v.widgets.hovered.weak_bg_fill = p.widget_hover;
    v.widgets.hovered.bg_fill = p.widget_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    v.widgets.active.rounding = r;
    // Achtung: `RichText::strong()` nimmt die Farbe von widgets.active.fg_stroke. Diese Schriftfarbe
    // deshalb nie aendern, sondern den Hintergrund: im hellen Design bekommt der gedrueckte Zustand
    // den hellblauen statt des kraeftigen Akzent-Hintergrunds.
    let pressed = if light { p.sel_bg } else { p.accent };
    v.widgets.active.weak_bg_fill = pressed;
    v.widgets.active.bg_fill = pressed;
    v.widgets.open.rounding = r;
    v.widgets.open.weak_bg_fill = p.widget_hover;
    v.widgets.open.bg_fill = p.widget_hover;
    style.visuals = v;
    style
}

// ---------------------------------------------------------------- Bausteine

fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(pal().card)
        .stroke(egui::Stroke::new(1.0_f32, pal().border))
        .rounding(14.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title.to_uppercase()).size(12.0).color(pal().muted).strong());
            ui.add_space(2.0);
            add(ui);
        });
}

fn dot(ui: &mut egui::Ui, color: Color32) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    ui.painter().circle_filled(r.center(), 6.5, color);
}

fn mb(b: u64) -> String {
    format!("{:.1} MB", b as f64 / 1_048_576.0)
}

fn mode_title(m: Mode) -> &'static str {
    match m {
        Mode::Normal => "Normal",
        Mode::Storage => "Storage",
        Mode::RapidCharge => "RapidCharge",
    }
}

fn mode_desc(m: Mode) -> &'static str {
    match m {
        Mode::Normal => "Der Akku wird bis 100 % geladen.",
        Mode::Storage => "Die Ladung ist auf ca. 80 % begrenzt. Das schont den Akku im Dauerbetrieb am Netz.",
        Mode::RapidCharge => "Schnellladen ist aktiv (kann hier nur angezeigt, nicht gesetzt werden).",
    }
}

fn mode_color(m: Mode) -> Color32 {
    match m {
        Mode::Normal => pal().accent,
        Mode::Storage => pal().green,
        Mode::RapidCharge => pal().amber,
    }
}

/// Hinweis zum Update: Rueckfrage bei neuer Version, Fortschritt, Fehler. Setzt `install`/`dismiss`.
fn update_card(ui: &mut egui::Ui, st: &UpState, install: &mut bool, dismiss: &mut bool) {
    let failed = matches!(st, UpState::Failed(_));
    if !failed && !matches!(st, UpState::Available(_)) && !st.busy() {
        return;
    }
    let (fill, accent) = if failed { (pal().err_bg, pal().red) } else { (pal().info_bg, pal().accent) };
    let text = if failed { pal().err_text } else { pal().info_text };
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0_f32, accent))
        .rounding(14.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match st {
                UpState::Available(r) => {
                    ui.label(RichText::new(format!("Neue Version {} verfuegbar", r.version)).size(16.0).strong());
                    ui.label(
                        RichText::new(format!(
                            "Installiert ist Version {}. Die neue Version wird von GitHub heruntergeladen, danach startet die App neu.",
                            update::CURRENT
                        ))
                        .size(12.5)
                        .color(text),
                    );
                    let notes: String = r.notes.lines().take(4).collect::<Vec<_>>().join("\n");
                    let notes: String = notes.chars().take(300).collect();
                    if !notes.trim().is_empty() {
                        ui.label(RichText::new(notes).size(12.5).color(pal().muted));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Jetzt aktualisieren").clicked() {
                            *install = true;
                        }
                        if ui.button("Spaeter").clicked() {
                            *dismiss = true;
                        }
                    });
                }
                UpState::Downloading { done, total } => {
                    ui.label(RichText::new("Update wird heruntergeladen").size(16.0).strong());
                    if *total > 0 {
                        ui.add(
                            egui::ProgressBar::new((*done as f32 / *total as f32).min(1.0))
                                .desired_height(10.0)
                                .fill(pal().accent),
                        );
                        ui.label(RichText::new(format!("{} von {}", mb(*done), mb(*total))).size(13.0));
                    } else {
                        ui.add(egui::ProgressBar::new(0.0).desired_height(10.0).fill(pal().accent).animate(true));
                        ui.label(RichText::new(format!("{} heruntergeladen", mb(*done))).size(13.0));
                    }
                }
                UpState::Installing | UpState::ReadyToRestart | UpState::Restarting => {
                    ui.label(RichText::new("Update wird installiert, die App startet gleich neu ...").size(16.0).strong());
                    ui.add(egui::ProgressBar::new(1.0).desired_height(10.0).fill(pal().accent).animate(true));
                }
                UpState::Failed(e) => {
                    ui.label(RichText::new("Update nicht moeglich").size(16.0).strong());
                    ui.label(RichText::new(e).size(13.0).color(text));
                    if ui.button("Schliessen").clicked() {
                        *dismiss = true;
                    }
                }
                _ => {}
            }
        });
    ui.add_space(8.0);
}

// ---------------------------------------------------------------- App

struct App {
    status: Result<Mode, String>,
    upd: Arc<Mutex<UpState>>,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        // Update-Pruefung beim Start: ohne Nachfrage und still; gefragt wird erst, wenn es etwas gibt
        update::cleanup();
        let upd = Arc::new(Mutex::new(UpState::Idle));
        update::check(upd.clone(), ctx.clone(), false);
        let mut app = App { status: Err(String::new()), upd };
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
        // Palette des aktuellen Systemdesigns (dunkel/hell) fuer diesen Frame
        IS_LIGHT.store(ctx.theme() == egui::Theme::Light, AtomicOrdering::Relaxed);

        // Update ist installiert: neue Version starten, diese beenden
        if matches!(*self.upd.lock().unwrap(), UpState::ReadyToRestart) {
            match update::restart() {
                Ok(()) => {
                    // Sofort merken: bis das Fenster wirklich zu ist, laufen evtl. noch Frames, und die
                    // duerfen die App nicht noch einmal starten
                    *self.upd.lock().unwrap() = UpState::Restarting;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                Err(e) => *self.upd.lock().unwrap() = UpState::Failed(format!("{e:#}")),
            }
        }
        if matches!(*self.upd.lock().unwrap(), UpState::Restarting) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let st = self.upd.lock().unwrap().clone();
        let current = self.status.as_ref().ok().copied();
        let mut target: Option<Mode> = None;
        let mut refresh = false;
        let mut install_upd = false;
        let mut dismiss_upd = false;
        let mut check_upd = false;

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(pal().bg).inner_margin(egui::Margin::same(18.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.label(RichText::new("Legion Battery").size(26.0).strong());
                    ui.label(RichText::new("Ladeverhalten des Akkus steuern").size(13.0).color(pal().muted));
                    ui.add_space(10.0);

                    update_card(ui, &st, &mut install_upd, &mut dismiss_upd);

                    // Aktueller Zustand
                    match &self.status {
                        Ok(mode) => card(ui, "Aktueller Modus", |ui| {
                            ui.horizontal(|ui| {
                                dot(ui, mode_color(*mode));
                                ui.label(RichText::new(mode_title(*mode)).size(22.0).strong());
                            });
                            ui.label(RichText::new(mode_desc(*mode)).size(13.0).color(pal().muted));
                        }),
                        Err(e) => {
                            egui::Frame::none()
                                .fill(pal().err_bg)
                                .stroke(egui::Stroke::new(1.0_f32, pal().red))
                                .rounding(14.0)
                                .inner_margin(egui::Margin::same(14.0))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(RichText::new("Status nicht lesbar").size(16.0).strong());
                                    if !e.is_empty() {
                                        ui.label(RichText::new(e).size(13.0).color(pal().err_text));
                                    }
                                });
                        }
                    }
                    ui.add_space(8.0);

                    // Modus waehlen
                    card(ui, "Modus waehlen", |ui| {
                        ui.add_enabled_ui(current.is_some(), |ui| {
                            let gap = ui.spacing().item_spacing.x;
                            let w = (ui.available_width() - gap) / 2.0;
                            ui.horizontal(|ui| {
                                for (m, label) in [(Mode::Normal, "Normal"), (Mode::Storage, "Storage (80 %)")] {
                                    let btn = egui::Button::new(RichText::new(label).size(15.0).strong())
                                        .min_size(egui::vec2(w, 40.0))
                                        .selected(current == Some(m));
                                    if ui.add(btn).clicked() && current != Some(m) {
                                        target = Some(m);
                                    }
                                }
                            });
                        });
                        ui.label(
                            RichText::new("Normal laedt bis 100 %. Storage begrenzt die Ladung auf ca. 80 % (Conservation).")
                                .size(12.5)
                                .color(pal().muted),
                        );
                    });
                    ui.add_space(8.0);

                    // Programm / Update
                    card(ui, "Programm", |ui| {
                        ui.label(RichText::new(format!("Version {}", update::CURRENT)).size(13.0).color(pal().muted));
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Status aktualisieren").clicked() {
                                refresh = true;
                            }
                            let checking = matches!(st, UpState::Checking);
                            if ui
                                .add_enabled(!checking && !st.busy(), egui::Button::new("Nach Updates suchen"))
                                .clicked()
                            {
                                check_upd = true;
                            }
                        });
                        match &st {
                            UpState::Checking => {
                                ui.label(RichText::new("Suche nach Updates ...").size(12.5).color(pal().muted));
                            }
                            UpState::UpToDate => {
                                ui.label(RichText::new("Die neueste Version ist installiert.").size(12.5).color(pal().green));
                            }
                            _ => {}
                        }
                    });
                });
            });

        if let Some(t) = target {
            self.set(t);
        }
        if refresh {
            self.refresh();
        }
        if install_upd {
            let rel = match &*self.upd.lock().unwrap() {
                UpState::Available(r) => Some(r.clone()),
                _ => None,
            };
            if let Some(r) = rel {
                update::install(self.upd.clone(), ctx.clone(), r);
            }
        }
        if dismiss_upd {
            *self.upd.lock().unwrap() = UpState::Idle;
        }
        if check_upd {
            update::check(self.upd.clone(), ctx.clone(), true);
        }
    }
}

/// Fenster-Icon (128x128 RGBA, aus assets/icon.rgba)
fn window_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!("../assets/icon.rgba").to_vec(),
        width: 128,
        height: 128,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 470.0])
            .with_min_inner_size([420.0, 380.0])
            .with_title(APP_TITLE)
            .with_icon(window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            setup_style(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}
