//! System paste — copy text to clipboard and send the paste keystroke.
//!
//! Uses arboard (clipboard) + enigo 0.6 (keyboard simulation).
//! Same behavior as Handy's "Direct" paste method.

use anyhow::Result;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use log::debug;

/// Copy text to the system clipboard.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to set clipboard: {}", e))?;
    Ok(())
}

/// The platform-appropriate "paste" key combo.
#[cfg(target_os = "macos")]
fn paste_keys() -> (Key, Key) {
    (Key::Meta, Key::Unicode('v'))
}

#[cfg(not(target_os = "macos"))]
fn paste_keys() -> (Key, Key) {
    (Key::Control, Key::Unicode('v'))
}

/// Send a paste keystroke (Cmd+V on macOS, Ctrl+V on Windows/Linux).
/// The text must already be on the clipboard.
pub fn send_paste_keystroke() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| anyhow::anyhow!("Failed to init enigo: {}", e))?;

    let (modifier, v_key) = paste_keys();

    // Press modifier
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| anyhow::anyhow!("Failed to press modifier: {}", e))?;

    // Press and release V
    enigo
        .key(v_key, Direction::Click)
        .map_err(|e| anyhow::anyhow!("Failed to send V key: {}", e))?;

    // Release modifier
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| anyhow::anyhow!("Failed to release modifier: {}", e))?;

    Ok(())
}

/// Send an Enter keypress.
pub fn send_enter() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| anyhow::anyhow!("Failed to init enigo: {}", e))?;

    enigo
        .key(Key::Return, Direction::Click)
        .map_err(|e| anyhow::anyhow!("Failed to send Enter: {}", e))?;

    Ok(())
}

/// Paste text using the specified method.
pub fn paste(text: &str, method: &str) -> Result<()> {
    debug!(
        "Pasting text (method={}): {}",
        method,
        &text[..text.len().min(50)]
    );

    match method {
        "copy_only" => {
            copy_to_clipboard(text)?;
        }
        "none" => {}
        _ => {
            // "direct" or "ctrl_v" — copy + paste keystroke
            copy_to_clipboard(text)?;
            send_paste_keystroke()?;
        }
    }

    Ok(())
}

/// Paste text and optionally send Enter for auto-submit.
pub fn paste_with_auto_submit(text: &str, method: &str, auto_submit: bool) -> Result<()> {
    paste(text, method)?;
    if auto_submit {
        send_enter()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paste_method_none() {
        // "none" should not error (it's a no-op)
        assert!(paste("test", "none").is_ok());
    }

    #[test]
    fn test_paste_method_copy_only() {
        // This might fail without a clipboard, but the function should be callable
        let _ = paste("test", "copy_only");
    }
}
