//! Transcription orchestration.
//!
//! This module owns the shared app state and coordinates the flow:
//!
//! 1. Hotkey pressed  → start recording (cpal input stream)
//! 2. Hotkey released → stop recording → encode WAV → resample to 16 kHz
//! 3. Spawn background thread → send to Groq API → get text back
//! 4. Paste text into the active app (on main thread, via the result channel)
//!
//! The background thread only does async I/O (HTTP). Pasting happens on the
//! main thread because enigo/keyboard simulation works best there.

use crate::{audio, groq_client::GroqClient, paste, settings::Settings, shortcut::HotkeyHandler};
use log::{debug, error, info};
use std::sync::mpsc::{self, Receiver, Sender};

/// What the app is currently doing. Drives the status display in the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Idle,
    Recording,
    Transcribing,
    /// The text from the last successful transcription (shown briefly).
    Done(String),
    Error(String),
}

/// Result sent from the background transcription thread back to the main
/// thread via the MPSC channel.
enum TranscriptionResult {
    Success(String),
    Error(String),
}

/// All the shared state the app needs during its lifetime.
pub struct AppState {
    pub settings: Settings,
    pub status: Status,
    pub audio: audio::AudioRecorder,
    pub hotkey_handler: Option<HotkeyHandler>,
    // Channel for background transcription results
    result_tx: Sender<TranscriptionResult>,
    result_rx: Receiver<TranscriptionResult>,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            settings,
            status: Status::Idle,
            audio: audio::AudioRecorder::new(),
            hotkey_handler: None,
            result_tx: tx,
            result_rx: rx,
        }
    }

    /// Register the push-to-talk hotkey. Must be called on the main thread.
    pub fn init_hotkeys(&mut self) -> Result<(), String> {
        match HotkeyHandler::new(&self.settings.shortcut) {
            Ok(handler) => {
                self.hotkey_handler = Some(handler);
                info!("Global hotkey registered: {}", self.settings.shortcut);
                Ok(())
            }
            Err(e) => {
                error!("Failed to init hotkeys: {}", e);
                Err(e)
            }
        }
    }

    /// Re-register the hotkey when the user changes the shortcut in settings.
    pub fn update_shortcut(&mut self, new_shortcut: &str) -> Result<(), String> {
        // Drop the old handler (unregisters the old hotkey via Drop).
        self.hotkey_handler = None;
        // Register the new one.
        match HotkeyHandler::new(new_shortcut) {
            Ok(handler) => {
                self.hotkey_handler = Some(handler);
                Ok(())
            }
            Err(e) => {
                // Restore the old handler if the new one fails.
                self.hotkey_handler = Some(HotkeyHandler::new(&self.settings.shortcut)?);
                Err(e)
            }
        }
    }

    /// Called every frame from the egui update loop.
    ///
    /// Polls for:
    /// - Hotkey press/release events → start/stop recording
    /// - Background transcription results → paste + update status
    pub fn poll(&mut self) {
        // --- Poll hotkey events ---
        if let Some(handler) = &self.hotkey_handler {
            if let Some(event) = handler.poll_event() {
                if event.is_press {
                    self.on_hotkey_press();
                } else {
                    self.on_hotkey_release();
                }
            }
        }

        // --- Poll transcription results from background thread ---
        // try_recv drains all pending results; we handle the latest one.
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                TranscriptionResult::Success(text) => {
                    debug!("Transcription succeeded: {} chars", text.len());
                    self.status = Status::Done(text.clone());

                    // Paste on the main thread (enigo works best here).
                    let method = self.settings.paste_method_str();
                    if let Err(e) =
                        paste::paste_with_auto_submit(&text, method, self.settings.auto_submit)
                    {
                        self.status = Status::Error(format!("Paste failed: {}", e));
                    }
                }
                TranscriptionResult::Error(msg) => {
                    error!("Transcription error: {}", msg);
                    self.status = Status::Error(msg);
                }
            }
        }
    }

    /// Hotkey pressed: start recording if not already recording.
    fn on_hotkey_press(&mut self) {
        if self.status == Status::Recording {
            return;
        }
        if self.status == Status::Transcribing {
            return;
        }

        match self.audio.start() {
            Ok(()) => {
                info!("Recording started");
                self.status = Status::Recording;
            }
            Err(e) => {
                error!("Failed to start recording: {}", e);
                self.status = Status::Error(format!("Audio: {}", e));
            }
        }
    }

    /// Hotkey released: stop recording and kick off transcription.
    fn on_hotkey_release(&mut self) {
        if self.status != Status::Recording {
            return;
        }

        let (samples, rate) = self.audio.stop();
        info!("Recording stopped: {} samples @ {}Hz", samples.len(), rate);
        self.status = Status::Transcribing;

        // Clone what the background thread needs.
        let tx = self.result_tx.clone();
        let api_key = self.settings.groq_api_key.clone();
        let model = self.settings.groq_model.clone();
        let language = self.settings.language.clone();

        std::thread::spawn(move || {
            // Each transcription gets its own tokio runtime.
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(TranscriptionResult::Error(format!(
                        "Failed to create tokio runtime: {}",
                        e
                    )));
                    return;
                }
            };

            rt.block_on(async move {
                // Resample to 16 kHz mono, then encode as WAV bytes.
                let resampled = audio::resample_to_16k(&samples, rate);
                let wav_bytes = audio::encode_wav(&resampled, 16000);

                debug!(
                    "WAV size: {} bytes ({} samples)",
                    wav_bytes.len(),
                    resampled.len()
                );

                let groq = GroqClient::default();
                match groq
                    .transcribe(&api_key, &model, &language, &wav_bytes)
                    .await
                {
                    Ok(text) => {
                        let _ = tx.send(TranscriptionResult::Success(text));
                    }
                    Err(e) => {
                        let _ = tx.send(TranscriptionResult::Error(e.to_string()));
                    }
                }
            });
        });
    }

    /// Save the current settings to disk.
    pub fn save_settings(&self) -> anyhow::Result<()> {
        let s = self.settings.clone();
        s.save()?;
        // Keep our in-memory copy in sync.
        Ok(())
    }

    /// Quick check — is the API key set?
    pub fn has_api_key(&self) -> bool {
        !self.settings.groq_api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transitions() {
        let settings = Settings::default();
        let mut state = AppState::new(settings);
        assert_eq!(state.status, Status::Idle);
    }
}
