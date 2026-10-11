//! Settings — a single JSON file in the user's config directory.
//!
//! Plain, simple, no Tauri store plugin. Everything the user configures
//! lives here.

use anyhow::{anyhow, Context};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which transcription provider to use.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[allow(dead_code)] // only one provider for V1, but kept for future expansion
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Groq,
}

/// How the dictation shortcut works.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    /// Hold the shortcut to record, release to stop.
    #[default]
    PushToTalk,
    /// Tap to start, tap again to stop.
    Toggle,
}

/// How the transcribed text gets into the active app.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PasteMethod {
    /// Copy to clipboard, then send Ctrl+V.
    #[default]
    Direct,
    /// Copy to clipboard, then send Ctrl+V (same but explicit).
    CtrlV,
    /// Copy to clipboard only — user pastes manually.
    CopyOnly,
    /// Do nothing (for debugging).
    None,
}

/// All settings the user can change. Saved to a single JSON file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    /// Groq API key. User pastes it here. Stored as plain text (local machine only).
    pub groq_api_key: String,
    /// Groq model to use. Default: "whisper-large-v3-turbo".
    pub groq_model: String,
    /// Language for transcription. "auto" lets Groq detect it.
    pub language: String,
    /// The global shortcut. Parsed by global-hotkey crate.
    pub shortcut: String,
    /// Push-to-talk or toggle.
    #[serde(default)]
    pub activation_mode: ActivationMode,
    /// How to paste the result.
    #[serde(default)]
    pub paste_method: PasteMethod,
    /// Automatically press Enter after pasting.
    #[serde(default = "default_true")]
    pub auto_submit: bool,
    /// Which key to send for auto-submit (e.g., "Enter").
    #[serde(default = "default_auto_submit_key")]
    pub auto_submit_key: String,
    /// Show the recording overlay?
    #[serde(default = "default_true")]
    pub show_overlay: bool,
    /// Start minimized (only in tray) on launch?
    #[serde(default)]
    pub start_hidden: bool,
    /// Start automatically when the user logs in?
    #[serde(default)]
    pub autostart: bool,
}

fn default_auto_submit_key() -> String {
    "Enter".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            groq_api_key: String::new(),
            groq_model: "whisper-large-v3-turbo".to_string(),
            language: "auto".to_string(),
            shortcut: "ctrl+shift+space".to_string(),
            activation_mode: ActivationMode::PushToTalk,
            paste_method: PasteMethod::Direct,
            auto_submit: true,
            auto_submit_key: "Enter".to_string(),
            show_overlay: true,
            start_hidden: false,
            autostart: false,
        }
    }
}

impl Settings {
    /// Where the settings file lives on disk.
    /// Linux:   ~/.config/handy-groq/settings.json
    /// macOS:   ~/Library/Application Support/handy-groq/settings.json
    /// Windows: %APPDATA%\handy-groq\settings.json
    pub fn config_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("handy-groq").join("settings.json")
    }

    /// Load settings from disk. If the file doesn't exist or is invalid,
    /// return defaults.
    pub fn load() -> Self {
        let path = Self::config_path();
        debug!("Loading settings from {}", path.display());

        match std::fs::read_to_string(&path) {
            Ok(data) => match serde_json::from_str::<Settings>(&data) {
                Ok(s) => {
                    debug!("Settings loaded successfully");
                    s
                }
                Err(e) => {
                    debug!("Settings JSON parse error: {}. Using defaults.", e);
                    Self::default()
                }
            },
            Err(_) => {
                debug!("No settings file found. Using defaults.");
                Self::default()
            }
        }
    }

    /// Save settings to disk. Creates the parent directory if needed.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        let dir = path
            .parent()
            .ok_or_else(|| anyhow!("Settings path has no parent"))?;

        std::fs::create_dir_all(dir).context("Failed to create config directory")?;

        let data = serde_json::to_string_pretty(self).context("Failed to serialize settings")?;

        std::fs::write(&path, data)
            .with_context(|| format!("Failed to write settings to {}", path.display()))?;

        debug!("Settings saved to {}", path.display());
        Ok(())
    }

    /// Convert the PasteMethod enum to the string format expected by `paste.rs`.
    pub fn paste_method_str(&self) -> &'static str {
        match self.paste_method {
            PasteMethod::Direct | PasteMethod::CtrlV => "direct",
            PasteMethod::CopyOnly => "copy_only",
            PasteMethod::None => "none",
        }
    }
}

/// Common ISO-639-1 language codes offered in the settings dropdown.
pub fn language_options() -> Vec<(&'static str, &'static str)> {
    vec![
        ("auto", "Auto-detect"),
        ("en", "English"),
        ("es", "Spanish"),
        ("fr", "French"),
        ("de", "German"),
        ("ja", "Japanese"),
        ("zh", "Chinese"),
        ("ko", "Korean"),
        ("it", "Italian"),
        ("pt", "Portuguese"),
        ("ru", "Russian"),
        ("ar", "Arabic"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert_eq!(s.groq_model, "whisper-large-v3-turbo");
        assert_eq!(s.shortcut, "ctrl+shift+space");
        assert!(s.auto_submit);
    }

    #[test]
    fn test_round_trip() {
        let mut s = Settings::default();
        s.groq_api_key = "test-key-123".to_string();
        s.language = "en".to_string();

        let json = serde_json::to_string(&s).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.groq_api_key, "test-key-123");
        assert_eq!(parsed.language, "en");
    }

    #[test]
    fn test_backward_compat_missing_fields() {
        // Simulate an older settings file with missing fields
        let old_json = r#"{"groq_api_key":"old-key","groq_model":"whisper-large-v3-turbo","language":"auto","shortcut":"Ctrl+Shift.Space"}"#;
        let parsed: Settings = serde_json::from_str(old_json).unwrap();
        // New fields should default
        assert_eq!(parsed.paste_method, PasteMethod::Direct);
        assert!(parsed.auto_submit);
    }
}
