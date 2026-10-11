//! egui UI rendering for handy-groq.
//!
//! One window: a status banner at the top + an editable settings form below.

use crate::settings::{language_options, PasteMethod, Settings};
use crate::transcription::{AppState, Status};
use eframe::egui::{self, Color32, RichText};
use log::debug;

/// Render the entire UI. Called once per frame from the egui update loop.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    // Rounded window corners (the default theme is fine, just add rounding).
    {
        let mut visual = egui::Visuals::light();
        visual.window_corner_radius = egui::CornerRadius::same(8);
        ui.ctx().set_visuals(visual);
    }

    // Title
    ui.label(RichText::new("handy-groq").heading());
    ui.label(
        RichText::new("Lean speech-to-text dictation via Groq Whisper API")
            .italics()
            .small(),
    );
    ui.add_space(12.0);

    // Status banner
    render_status(ui, &state.status);
    ui.add_space(16.0);

    // Settings form
    render_settings(ui, state);

    ui.add_space(16.0);

    // Action buttons
    ui.horizontal(|ui| {
        if ui.button("💾 Save Settings").clicked() {
            match state.save_settings() {
                Ok(()) => debug!("Saved"),
                Err(e) => {
                    state.status = Status::Error(e.to_string());
                }
            }
            // Re-register the hotkey (in case the shortcut changed).
            // Clone first to avoid borrow conflict.
            let shortcut = state.settings.shortcut.clone();
            if let Err(e) = state.update_shortcut(&shortcut) {
                state.status = Status::Error(e);
            }
        }

        if ui.button("🔄 Reload").clicked() {
            let loaded = Settings::load();
            state.settings = loaded;
            let shortcut = state.settings.shortcut.clone();
            if let Err(e) = state.update_shortcut(&shortcut) {
                state.status = Status::Error(e);
            }
            state.status = Status::Idle;
        }
    });

    // Request a repaint so we can poll for hotkey events every frame.
    ui.ctx().request_repaint();
}

fn render_status(ui: &mut egui::Ui, status: &Status) {
    let (icon, text, color) = match status {
        Status::Idle => (
            "✓",
            "Ready — press the shortcut to record".to_string(),
            Color32::LIGHT_GREEN,
        ),
        Status::Recording => ("🔴", "Recording…".to_string(), Color32::LIGHT_RED),
        Status::Transcribing => ("⏳", "Transcribing…".to_string(), Color32::LIGHT_BLUE),
        Status::Done(text) => {
            let preview = if text.len() > 80 {
                format!("“{}”…", &text[..80])
            } else {
                format!("“{}”", text)
            };
            ("✓", preview, Color32::LIGHT_GREEN)
        }
        Status::Error(msg) => ("✗", msg.clone(), Color32::LIGHT_RED),
    };

    ui.group(|ui| {
        ui.label(
            RichText::new(format!("{} {}", icon, text))
                .color(color)
                .size(14.0),
        );
    });
}

fn render_settings(ui: &mut egui::Ui, state: &mut AppState) {
    ui.group(|ui| {
        ui.label(RichText::new("Groq API Key").strong());
        ui.add(
            egui::TextEdit::singleline(&mut state.settings.groq_api_key)
                .hint_text("Paste your Groq API key here (sk-...)"),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Get one at console.groq.com — required for transcription")
                .italics()
                .small(),
        );
        if !state.has_api_key() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("⚠ No API key set — transcription won't work until you save one")
                    .color(Color32::YELLOW),
            );
        }

        ui.add_space(12.0);
        ui.label(RichText::new("Language").strong());
        egui::ComboBox::from_label("language")
            .selected_text(language_name(&state.settings.language))
            .show_ui(ui, |ui| {
                for (code, name) in language_options() {
                    if ui.selectable_label(state.settings.language == code, name).clicked() {
                        state.settings.language = code.to_string();
                    }
                }
            });

        ui.add_space(8.0);
        ui.label(RichText::new("Push-to-talk Shortcut").strong());
        ui.add(egui::TextEdit::singleline(&mut state.settings.shortcut));
        ui.label(
            RichText::new(
                "Format: ctrl+shift+space (modifiers + key, joined by +). Takes effect after saving.",
            )
            .italics()
            .small(),
        );

        ui.add_space(8.0);
        ui.label(RichText::new("Paste Method").strong());
        egui::ComboBox::from_label("paste_method")
            .selected_text(paste_method_label(state.settings.paste_method))
            .show_ui(ui, |ui| {
                ui.selectable_label(
                    state.settings.paste_method == PasteMethod::Direct,
                    "Copy + Ctrl+V (auto-paste)",
                )
                .clicked()
                .then(|| state.settings.paste_method = PasteMethod::Direct);
                ui.selectable_label(
                    state.settings.paste_method == PasteMethod::CopyOnly,
                    "Copy only (manual paste)",
                )
                .clicked()
                .then(|| state.settings.paste_method = PasteMethod::CopyOnly);
                ui.selectable_label(
                    state.settings.paste_method == PasteMethod::None,
                    "None (debugging)",
                )
                .clicked()
                .then(|| state.settings.paste_method = PasteMethod::None);
            });

        ui.add_space(8.0);
        ui.checkbox(&mut state.settings.auto_submit, "Auto-press Enter after pasting");
    });
}

fn language_name(code: &str) -> String {
    let opts = language_options();
    for (c, name) in &opts {
        if c == &code {
            return name.to_string();
        }
    }
    code.to_string()
}

fn paste_method_label(method: PasteMethod) -> &'static str {
    match method {
        PasteMethod::Direct | PasteMethod::CtrlV => "Copy + Ctrl+V",
        PasteMethod::CopyOnly => "Copy only",
        PasteMethod::None => "None",
    }
}
