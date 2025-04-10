use eframe::{egui::{self, FontData, FontDefinitions, FontFamily, FontId, Visuals, Style, TextEdit}, App, CreationContext, NativeOptions};
use egui::Context;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
pub struct LTResponse {
    matches: Vec<LTMatch>,
}

#[derive(Deserialize, Debug)]
pub struct LTMatch {
    message: String,
    offset: usize,
    length: usize,
    replacements: Vec<LTSuggestion>,
}

#[derive(Deserialize, Debug)]
pub struct LTSuggestion {
    value: String,
}

pub struct NoteApp {
    note_content: String,
    selected_file: Option<String>,
    suggestions: Vec<LTMatch>,
    show_menu: bool,
}

impl NoteApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        apply_custom_style(&cc.egui_ctx);
        Self {
            note_content: String::new(),
            selected_file: None,
            suggestions: Vec::new(),
            show_menu: false,
        }
    }

    pub fn load_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.note_content = content;
                self.selected_file = path.file_name().and_then(|s| s.to_str()).map(String::from);
            }
        }
    }

    pub fn save_file(&self) {
        if let Some(filename) = &self.selected_file {
            let path = format!("notes/{}", filename);
            if let Err(err) = std::fs::write(&path, &self.note_content) {
                eprintln!("Failed to save file: {}", err);
            }
        } else if let Some(path) = rfd::FileDialog::new().save_file() {
            if let Err(err) = std::fs::write(path, &self.note_content) {
                eprintln!("Failed to save file: {}", err);
            }
        }
    }

    pub fn check_suggestions(&mut self) {
        let client = reqwest::blocking::Client::new();
        let res = client
            .post("https://api.languagetoolplus.com/v2/check")
            .form(&[
                ("text", self.note_content.as_str()),
                ("language", "en-US"),
            ])
            .send();

        match res {
            Ok(resp) => {
                if let Ok(parsed) = resp.json::<LTResponse>() {
                    self.suggestions = parsed.matches;
                }
            }
            Err(err) => {
                eprintln!("Suggestion error: {}", err);
            }
        }
    }
}

impl App for NoteApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Dropdown Menu
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("☰ Menu").clicked() {
                    self.show_menu = !self.show_menu;
                }
                if self.show_menu {
                    if ui.button("📂 Open File").clicked() {
                        self.load_file();
                        self.show_menu = false;
                    }
                    if ui.button("💾 Save File").clicked() {
                        self.save_file();
                        self.show_menu = false;
                    }
                    if ui.button("🔍 Check Grammar").clicked() {
                        self.check_suggestions();
                        self.show_menu = false;
                    }
                }
            });
        });

        // Main text editor
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_sized(
                ui.available_size(),
                TextEdit::multiline(&mut self.note_content)
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
        });

        // Suggestions panel
        if !self.suggestions.is_empty() {
            egui::Window::new("💡 Suggestions")
                .default_width(300.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    for suggestion in &self.suggestions {
                        let snippet = &self.note_content
                            [suggestion.offset..suggestion.offset + suggestion.length];
                        let replacement = suggestion
                            .replacements
                            .get(0)
                            .map(|r| r.value.as_str())
                            .unwrap_or("❌");
                        let suggestion_text = format!("{} → {}", snippet, replacement);

                        if ui.button(suggestion_text).clicked() {
                            self.note_content.replace_range(
                                suggestion.offset..suggestion.offset + suggestion.length,
                                replacement,
                            );
                            self.check_suggestions();
                            break;
                        }
                    }
                });
        }
    }
}

fn apply_custom_style(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "Minigap".to_owned(),
        FontData::from_owned(
            std::fs::read("fonts/Minigap-Regular.ttf").expect("Font file not found"),
        ),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Minigap".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "Minigap".to_owned());

    ctx.set_fonts(fonts);

    let mut style: Style = (*ctx.style()).clone();
    style.visuals = Visuals::dark();
    style.text_styles = [
        (egui::TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
        (egui::TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(16.0, FontFamily::Monospace)),
        (egui::TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (egui::TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
    ]
        .into();

    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        ..Default::default()
    };
    eframe::run_native(
        "Rust Note App",
        options,
        Box::new(|cc| Box::new(NoteApp::new(cc))),
    )
}
