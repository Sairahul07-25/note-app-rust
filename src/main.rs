use eframe::{
    egui::{self, FontData, FontDefinitions, FontFamily, Style, TextStyle, FontId, Visuals},
    App, CreationContext, NativeOptions,
};
use egui::{CentralPanel, Context, SidePanel, TopBottomPanel};
use serde::Deserialize;
use std::sync::Arc;
use egui::text::LayoutJob;
use egui::{TextFormat, Color32, Stroke};
use egui::{ RichText, ScrollArea,
};

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
        if !path.exists() {
            if let Err(e) = std::fs::create_dir(path) {
                eprintln!("Failed to create notes folder: {}", e);
                return;
            }
        }
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
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        // TODO: Trigger file open
                    }
                    if ui.button("Save").clicked() {
                        self.save_current_file();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.label("Undo/Redo coming soon");
                });
                ui.menu_button("View", |ui| {
                    ui.label("Theme: Dark");
                });
            });
        });
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

                // 📂 File Explorer: Open any file
                if ui.button("📂 Open File...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            self.note_content = content;
                            self.selected_file = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .map(String::from);
                        }
                    }
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

            // Save button
            if ui.button("💾 Save").clicked() {
                self.save_current_file();
            }

            ui.separator();

            // Prepare fonts and styles
            let base_format = TextFormat {
                font_id: FontId::monospace(16.0),
                color: Color32::WHITE,
                ..Default::default()
            };

            let highlight_format = TextFormat {
                font_id: FontId::monospace(16.0),
                color: Color32::WHITE,
                background: Color32::DARK_RED,
                ..Default::default()
            };

            // Create a scrollable area
            ScrollArea::vertical().show(ui, |ui| {
                let lines: Vec<&str> = self.note_content.lines().collect();
                let mut job = LayoutJob::default();
                let mut offset = 0;

                for (i, line) in lines.iter().enumerate() {
                    // Line number
                    let line_number = format!("{:>4} │ ", i + 1);
                    job.append(&line_number, 0.0, TextFormat {
                        font_id: FontId::monospace(16.0),
                        color: Color32::GRAY,
                        ..Default::default()
                    });

                    let mut cursor = 0;
                    while cursor < line.len() {
                        let mut matched = false;

                        for suggestion in &self.suggestions {
                            if suggestion.offset >= offset
                                && suggestion.offset < offset + line.len()
                                && suggestion.offset - offset == cursor
                            {
                                let rel_offset = suggestion.offset - offset;
                                let len = suggestion.length.min(line.len() - rel_offset);
                                let text = &line[rel_offset..rel_offset + len];

                                job.append(text, 0.0, highlight_format.clone());
                                cursor += len;
                                matched = true;
                                break;
                            }
                        }

                        if !matched {
                            let ch = &line[cursor..cursor + 1];
                            job.append(ch, 0.0, base_format.clone());
                            cursor += 1;
                        }
                    }

                    job.append("\n", 0.0, base_format.clone());
                    offset += line.len() + 1; // +1 for \n
                }

                ui.label(job);
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
                    // 🔁 Replace this whole for-loop with the new suggestion block:
                    for suggestion in &self.suggestions {
                        let snippet = &self.note_content[suggestion.offset..suggestion.offset + suggestion.length];
                        let replacement = suggestion.replacements.get(0).map(|r| r.value.as_str()).unwrap_or("❌");
                        let suggestion_text = format!("{} → {}", snippet, replacement);

                        if ui.button(suggestion_text).clicked() {
                            if let Some(replacement) = suggestion.replacements.get(0) {
                                self.note_content.replace_range(
                                    suggestion.offset..suggestion.offset + suggestion.length,
                                    &replacement.value,
                                );
                                self.check_suggestions(); // refresh
                                break; // avoid borrowing errors
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
