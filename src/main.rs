use eframe::{
    egui::{self, FontData, FontDefinitions, FontFamily, Style, TextStyle, FontId, Visuals},
    App, CreationContext, NativeOptions,
};
use egui::{CentralPanel, Context, SidePanel, TopBottomPanel};
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
    files: Vec<String>,
    selected_file: Option<String>,
    suggestions: Vec<LTMatch>,
}

impl NoteApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        apply_custom_style(&cc.egui_ctx);

        let mut app = Self {
            note_content: String::new(),
            files: Vec::new(),
            selected_file: None,
            suggestions: Vec::new(),
        };
        app.load_file_list();
        app
    }

    pub fn load_file_list(&mut self) {
        let path = std::path::Path::new("notes/");
        if let Ok(entries) = std::fs::read_dir(path) {
            self.files = entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            self.files.sort();
        }
    }

    pub fn load_selected_file(&mut self) {
        if let Some(filename) = &self.selected_file {
            let path = format!("notes/{}", filename);
            if let Ok(content) = std::fs::read_to_string(path) {
                self.note_content = content;
            }
        }
    }

    pub fn save_current_file(&self) {
        if let Some(filename) = &self.selected_file {
            let path = format!("notes/{}", filename);
            if let Err(err) = std::fs::write(&path, &self.note_content) {
                eprintln!("Failed to save file {}: {}", filename, err);
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
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.label("📓 Note Taking App with Suggestions");
        });

        SidePanel::left("file_panel")
            .resizable(true)
            .default_width(150.0)
            .show(ctx, |ui| {
                ui.heading("📁 Notes");

                if ui.button("🔄 Refresh").clicked() {
                    self.load_file_list();
                }

                let file_list = self.files.clone();
                for file in file_list {
                    let label = if Some(&file) == self.selected_file.as_ref() {
                        format!("👉 {}", file)
                    } else {
                        file.clone()
                    };
                    if ui.button(label).clicked() {
                        self.selected_file = Some(file.clone());
                        self.load_selected_file();
                    }
                }
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("📝 Editor");

            if ui.button("💾 Save").clicked() {
                self.save_current_file();
            }

            let available_size = ui.available_size(); // get full size of central panel

            // Expand multiline text box to fill area
            ui.allocate_ui_with_layout(available_size, egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.note_content)
                        .desired_rows(40)
                        .desired_width(f32::INFINITY),
                );
            });
        });



        SidePanel::right("suggestions_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("💡 Suggestions");

                if ui.button("🔍 Check Grammar").clicked() {
                    self.check_suggestions();
                }

                if self.suggestions.is_empty() {
                    ui.label("No suggestions yet.");
                } else {
                    for suggestion in &self.suggestions {
                        let suggestion_text = format!(
                            "{} → {}",
                            &self.note_content[suggestion.offset..suggestion.offset + suggestion.length],
                            suggestion.replacements.get(0).map_or("❌", |r| r.value.as_str())
                        );

                        if ui.button(suggestion_text).clicked() {
                            if let Some(replacement) = suggestion.replacements.get(0) {
                                self.note_content.replace_range(
                                    suggestion.offset..suggestion.offset + suggestion.length,
                                    &replacement.value,
                                );
                                self.check_suggestions(); // Refresh suggestions
                                break;
                            }
                        }
                    }
                }
            });
    }
}

fn apply_custom_style(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "Minigap".to_owned(),
        Arc::new(FontData::from_owned(
            std::fs::read("fonts/Minigap-Regular.ttf").expect("Font file not found"),
        )),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Minigap".to_owned());

    ctx.set_fonts(fonts);

    let mut style: Style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(10.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(8.0, FontFamily::Proportional)),
    ]
        .into();

    style.visuals = Visuals::dark();

    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Rust Note App",
        options,
        Box::new(|cc| Ok(Box::new(NoteApp::new(cc)))),
    )
}
