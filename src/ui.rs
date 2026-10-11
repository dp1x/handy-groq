//! egui UI rendering for handy-groq.
//!
//! Industrial control-panel aesthetic: simple gray base, flat buttons,
//! visual grouping of settings, waveform feedback during recording.

use crate::settings::{language_options, PasteMethod, Settings};
use crate::transcription::{AppState, Status};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use log::debug;

/// Return the industrial gray visual theme.
/// Old Windows control-panel feel: warm gray backgrounds, flat buttons,
/// simple borders, muted blue accent. No shadows, minimal rounding.
pub fn industrial_gray_theme() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();

    // Gray base — not white, not black. Warm industrial gray.
    visuals.panel_fill = Color32::from_rgb(0xe8, 0xe8, 0xe8);
    visuals.window_fill = Color32::from_rgb(0xf0, 0xf0, 0xf0);
    visuals.window_corner_radius = CornerRadius::same(4);
    visuals.override_text_color = Some(Color32::from_rgb(0x33, 0x33, 0x33));

    // Subtle grays for input backgrounds (industrial control-panel feel).
    visuals.faint_bg_color = Color32::from_rgb(0xf8, 0xf8, 0xf8);
    visuals.extreme_bg_color = Color32::from_rgb(0xe0, 0xe0, 0xe0);
    visuals.text_edit_bg_color = Some(Color32::from_rgb(0xff, 0xff, 0xff));
    visuals.code_bg_color = Color32::from_rgb(0xf0, 0xf0, 0xf0);

    // Flat, industrial button styles — no 3D bevels.
    let border = Color32::from_rgb(0xa0, 0xa0, 0xa0);
    let border_dark = Color32::from_rgb(0x80, 0x80, 0x80);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(0xf4, 0xf4, 0xf4);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0xe0, 0xe0, 0xe0);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xd4, 0xd4, 0xd4);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, border_dark);

    visuals.widgets.active.bg_fill = Color32::from_rgb(0xc0, 0xc0, 0xc0);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x70, 0x70, 0x70));

    // No drop shadows — flat industrial.
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow::NONE;

    visuals
}

/// Render the entire UI. Called once per frame from the egui update loop.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    // Title bar
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("handy-groq").heading());
        ui.label(
            RichText::new("Speech-to-text dictation via Groq Whisper API")
                .italics()
                .small(),
        );
    });
    ui.add_space(16.0);

    // Status banner (with waveform during recording)
    render_status(ui, &state.status, state);
    ui.add_space(24.0);

    // Settings form — grouped sections
    render_settings(ui, state);

    ui.add_space(16.0);

    // Action buttons
    ui.horizontal(|ui| {
        if ui.button("💾 Save Settings").clicked() {
            match state.save_settings() {
                Ok(()) => debug!("Settings saved"),
                Err(e) => state.status = Status::Error(e.to_string()),
            }
            // Re-register the hotkey (in case the shortcut changed).
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

    // Repaint every frame so we can poll hotkey events continuously.
    ui.ctx().request_repaint();
}

fn render_status(ui: &mut egui::Ui, status: &Status, state: &AppState) {
    let (icon, text, color) = match status {
        Status::Idle => (
            "✓",
            "Ready — press the shortcut to record".to_string(),
            Color32::from_rgb(0x4a, 0x90, 0x4a),
        ),
        Status::Recording => {
            let elapsed = state
                .elapsed_recording()
                .map(|s| format!(" ({:.1}s)", s))
                .unwrap_or_default();
            (
                "🔴",
                format!("Recording…{}", elapsed),
                Color32::from_rgb(0xd9, 0x53, 0x4f),
            )
        }
        Status::Transcribing => (
            "⏳",
            "Transcribing…".to_string(),
            Color32::from_rgb(0x33, 0x7a, 0xb7),
        ),
        Status::Done(text) => {
            let preview = if text.len() > 80 {
                format!("“{}”…", &text[..80])
            } else {
                format!("“{}”", text)
            };
            ("✓", preview, Color32::from_rgb(0x4a, 0x90, 0x4a))
        }
        Status::Error(msg) => ("✗", msg.clone(), Color32::from_rgb(0xd9, 0x53, 0x4f)),
    };

    ui.group(|ui| {
        ui.label(
            RichText::new(format!("{} {}", icon, text))
                .color(color)
                .size(14.0),
        );

        // Waveform visualization during recording
        if matches!(status, Status::Recording) {
            render_waveform(ui, state);
        }
    });
}

/// Draw a bar graph of recent RMS amplitudes — copied from Handy's
/// recording overlay concept, adapted for egui (9-bar industrial style).
fn render_waveform(ui: &mut egui::Ui, state: &AppState) {
    let levels = state.rms_levels(20);
    if levels.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new("🔊 Listening...").size(12.0));
        return;
    }

    let bar_width = 8.0;
    let gap = 3.0;
    let max_height = 30.0;
    let total_width = (bar_width + gap) * levels.len() as f32 - gap;

    ui.add_space(8.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("🔊 Listening...").size(12.0));
        ui.add_space(4.0);

        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(total_width, max_height), egui::Sense::hover());

        for (i, &level) in levels.iter().enumerate() {
            let bar_height = (level * 100.0).clamp(1.0, max_height);
            let x = rect.left() + i as f32 * (bar_width + gap);
            let y = rect.bottom() - bar_height;
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(bar_width, bar_height)),
                egui::CornerRadius::same(1),
                Color32::from_rgb(0x5b, 0x7f, 0xd4),
            );
        }
    });
}

fn render_settings(ui: &mut egui::Ui, state: &mut AppState) {
    // --- API Key section ---
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
                    .color(Color32::from_rgb(0xa0, 0x80, 0x00)),
            );
        }
    });
    ui.add_space(12.0);
    ui.separator();

    // --- Language + Shortcut section ---
    ui.group(|ui| {
        ui.label(RichText::new("Language").strong());
        egui::ComboBox::from_label("language")
            .selected_text(language_name(&state.settings.language))
            .show_ui(ui, |ui| {
                for (code, name) in language_options() {
                    if ui
                        .selectable_label(state.settings.language == code, name)
                        .clicked()
                    {
                        state.settings.language = code.to_string();
                    }
                }
            });

        ui.add_space(8.0);
        ui.label(RichText::new("Push-to-talk Shortcut").strong());
        ui.add(egui::TextEdit::singleline(&mut state.settings.shortcut));
        ui.label(
            RichText::new("Format: ctrl+shift+space. Takes effect after saving.")
                .italics()
                .small(),
        );
    });
    ui.add_space(12.0);
    ui.separator();

    // --- Paste + Behavior section ---
    ui.group(|ui| {
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
        ui.checkbox(
            &mut state.settings.auto_submit,
            "Auto-press Enter after pasting",
        );

        ui.add_space(8.0);
        if state.has_sound() {
            ui.label(RichText::new("🔊 Sound effects: on").small());
        } else {
            ui.label(
                RichText::new("🔇 Sound effects: off (no output device)")
                    .color(Color32::from_rgb(0xa0, 0x80, 0x00))
                    .small(),
            );
        }
    });
    ui.add_space(12.0);
    ui.separator();

    // --- Devices section ---
    ui.group(|ui| {
        ui.label(RichText::new("Microphone").strong());
        egui::ComboBox::from_label("mic")
            .selected_text(if state.settings.selected_mic.is_empty() {
                "Default (system)".to_string()
            } else {
                state.settings.selected_mic.clone()
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(state.settings.selected_mic.is_empty(), "Default (system)")
                    .clicked()
                {
                    state.settings.selected_mic = String::new();
                }
                for name in &state.input_devices {
                    if ui
                        .selectable_label(state.settings.selected_mic == *name, name)
                        .clicked()
                    {
                        state.settings.selected_mic = name.clone();
                    }
                }
                if state.input_devices.is_empty() {
                    ui.label(
                        RichText::new("No additional devices found")
                            .italics()
                            .small(),
                    );
                }
            });

        ui.add_space(8.0);
        ui.label(RichText::new("Output Device (sound effects)").strong());
        egui::ComboBox::from_label("output")
            .selected_text(if state.settings.selected_output.is_empty() {
                "Default (system)".to_string()
            } else {
                state.settings.selected_output.clone()
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        state.settings.selected_output.is_empty(),
                        "Default (system)",
                    )
                    .clicked()
                {
                    state.settings.selected_output = String::new();
                }
                for name in &state.output_devices {
                    if ui
                        .selectable_label(state.settings.selected_output == *name, name)
                        .clicked()
                    {
                        state.settings.selected_output = name.clone();
                    }
                }
                if state.output_devices.is_empty() {
                    ui.label(
                        RichText::new("No additional devices found")
                            .italics()
                            .small(),
                    );
                }
            });
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
