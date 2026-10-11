//! Sound effects — WAV files copied from the original Handy project.
//!
//! Plays short UI feedback sounds (marimba for record start/stop, pop for
//! streaming transitions) via `rodio`. Gracefully no-ops if no audio output
//! device is available (e.g. in CI or headless environments).

use anyhow::Result;
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub struct SoundPlayer {
    _stream: OutputStream,
    sink: Sink,
}

impl SoundPlayer {
    pub fn try_new() -> Result<Self> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| anyhow::anyhow!("no audio output device: {}", e))?;
        let sink = Sink::try_new(&handle)?;
        Ok(Self {
            _stream: stream,
            sink,
        })
    }

    fn play(&self, wav: &'static [u8]) {
        match Decoder::new(Cursor::new(wav)) {
            Ok(source) => self.sink.append(source),
            Err(e) => log::warn!("failed to decode sound effect: {}", e),
        }
    }

    /// Marimba note — played when push-to-talk starts recording.
    pub fn play_record_start(&self) {
        self.play(include_bytes!("../resources/marimba_start.wav"));
    }

    /// Marimba note — played when push-to-talk stops recording.
    pub fn play_record_stop(&self) {
        self.play(include_bytes!("../resources/marimba_stop.wav"));
    }
}
