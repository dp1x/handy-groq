//! Global keyboard shortcut management.
//!
//! Uses the `global-hotkey` crate (same one Tauri uses under the hood) to
//! register a system-wide push-to-talk hotkey. The manager must be created
//! on the main thread (required on macOS).

use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use log::debug;
use std::str::FromStr;

/// Wraps the global-hotkey manager + the single registered hotkey.
///
/// The `manager` field is never read directly — it stays alive for the
/// lifetime of the struct so the registered hotkey remains active. Dropping
/// the manager unregisters all hotkeys.
pub struct HotkeyHandler {
    #[allow(dead_code)]
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
}

/// A polled hotkey event — press or release of our registered shortcut.
pub struct HotkeyEvent {
    pub is_press: bool,
}

impl HotkeyHandler {
    /// Create a manager and register the given shortcut string.
    ///
    /// The string uses the `global-hotkey` format, e.g. `"ctrl+shift+space"`.
    /// Must be called on the main thread.
    pub fn new(shortcut: &str) -> Result<Self, String> {
        let hotkey = HotKey::from_str(shortcut)
            .map_err(|e| format!("Shortcut '{}' is not valid: {}", shortcut, e))?;

        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Failed to start global hotkey manager: {}", e))?;

        manager
            .register(hotkey)
            .map_err(|e| format!("Failed to register shortcut '{}': {}", shortcut, e))?;

        debug!("Registered hotkey: {} (id={})", shortcut, hotkey.id());
        Ok(Self { manager, hotkey })
    }

    /// Poll for events. Returns `Some` only if our hotkey fired.
    ///
    /// This drains the process-global event receiver. Events for other
    /// (unrelated) hotkeys are silently discarded — fine for V1 since we
    /// only register one.
    pub fn poll_event(&self) -> Option<HotkeyEvent> {
        let receiver = GlobalHotKeyEvent::receiver();
        while let Ok(event) = receiver.try_recv() {
            if event.id == self.hotkey.id() {
                debug!("Hotkey event: id={}, state={:?}", event.id, event.state);
                return Some(HotkeyEvent {
                    is_press: event.state == HotKeyState::Pressed,
                });
            }
        }
        None
    }

    /// The numeric ID assigned to this hotkey.
    #[allow(dead_code)]
    pub fn hotkey_id(&self) -> u32 {
        self.hotkey.id()
    }
}

impl Drop for HotkeyHandler {
    fn drop(&mut self) {
        // Dropping the manager unregisters the hotkey automatically.
        // Log only at debug level; errors here are non-fatal during shutdown.
        debug!("Dropping HotkeyHandler (id={})", self.hotkey.id());
    }
}

/// Parse a shortcut string without registering it (used for validation in UI).
#[allow(dead_code)]
pub fn parse_shortcut(s: &str) -> Result<(), String> {
    HotKey::from_str(s)
        .map(|hk| {
            debug!("Parsed shortcut '{}' → id={}", s, hk.id());
        })
        .map_err(|e| format!("Invalid shortcut: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut_valid() {
        // ctrl+shift+space is the default
        assert!(parse_shortcut("ctrl+shift+space").is_ok());
    }

    #[test]
    fn test_parse_shortcut_no_mods() {
        assert!(parse_shortcut("space").is_ok());
    }

    #[test]
    fn test_parse_shortcut_invalid() {
        // "shift+KeyQ+alt" — modifier after key is invalid format
        assert!(parse_shortcut("shift+KeyQ+alt").is_err());
    }

    #[test]
    fn test_parse_shortcut_empty() {
        assert!(parse_shortcut("").is_err());
    }
}
