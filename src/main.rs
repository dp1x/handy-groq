//! handy-groq — lean speech-to-text dictation via Groq Whisper API.
//!
//! A fork of Handy, stripped down:
//! - No Tauri, no WebKit — just eframe/egui (pure Rust GUI)
//! - No local models — transcription goes to Groq's Whisper API (BYOK)
//! - Single JSON settings file in the user's config directory
//! - Global push-to-talk keyboard shortcut (like Handy)
//!
//! Build: `cargo build` (or use GitHub Actions — see .github/workflows/)
//! Docs:  README.md, BUILD.md, CONTRIBUTING.md

use eframe::egui;
use std::cell::RefCell;
use std::rc::Rc;

mod audio;
mod groq_client;
mod paste;
mod settings;
mod shortcut;
mod transcription;
mod ui;

use transcription::AppState;

/// The eframe application. Owns the shared state.
/// Uses Rc<RefCell> since everything runs on the main thread — no
/// cross-thread sharing needed (background threads receive cloned data only).
struct HandyGroqApp {
    state: Rc<RefCell<AppState>>,
}

impl eframe::App for HandyGroqApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // One borrow per frame: poll events, then render.
        let mut state = self.state.borrow_mut();
        state.poll();
        ui::render(ui, &mut state);
    }
}

fn main() -> eframe::Result<()> {
    // Log to stderr — helps with debugging via RUST_LOG=debug.
    env_logger::init();

    // Load settings (single JSON file in config dir).
    let settings = settings::Settings::load();

    // Create shared state (single-threaded — no Arc/Mutex needed).
    let state = Rc::new(RefCell::new(AppState::new(settings)));

    // Register the global hotkey BEFORE launching the egui event loop.
    // (global-hotkey requires the main thread on some platforms.)
    {
        let mut s = state.borrow_mut();
        if let Err(e) = s.init_hotkeys() {
            eprintln!("⚠  Failed to register global shortcut: {}", e);
            eprintln!("⚠  The app will run, but push-to-talk won't work.");
            eprintln!("⚠  Check that no other app is using the shortcut.");
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("handy-groq")
            .with_inner_size([520.0, 480.0])
            .with_min_inner_size([400.0, 360.0]),
        ..Default::default()
    };

    eframe::run_native(
        "handy-groq",
        native_options,
        Box::new(|_cc| Ok(Box::new(HandyGroqApp { state }))),
    )
}
