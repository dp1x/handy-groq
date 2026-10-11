//! Groq Whisper API client.
//!
//! Sends audio to Groq's OpenAI-compatible speech-to-text endpoint.
//! User provides their own API key (BYOK). No models run locally.

use log::debug;
use serde::Deserialize;

/// Response from Groq's /audio/transcriptions endpoint.
#[derive(Deserialize)]
struct GroqResponse {
    text: String,
}

/// Error categories for Groq API failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroqError {
    /// No API key configured.
    NoApiKey,
    /// 401 — the API key is invalid.
    InvalidApiKey,
    /// 429 — rate limited by Groq.
    RateLimited,
    /// Network error or non-OK response.
    Network(String),
    /// The response body couldn't be parsed.
    ParseError,
}

impl std::fmt::Display for GroqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroqError::NoApiKey => {
                write!(f, "No Groq API key set. Paste one in Settings.")
            }
            GroqError::InvalidApiKey => {
                write!(f, "Groq API key is invalid. Check it in Settings.")
            }
            GroqError::RateLimited => {
                write!(f, "Rate limited by Groq. Wait a moment and try again.")
            }
            GroqError::Network(msg) => {
                write!(f, "Network error contacting Groq: {}", msg)
            }
            GroqError::ParseError => {
                write!(f, "Could not parse Groq response.")
            }
        }
    }
}

impl std::error::Error for GroqError {}

/// A lightweight client for Groq's speech-to-text API.
#[derive(Clone)]
pub struct GroqClient {
    client: reqwest::Client,
}

impl Default for GroqClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("handy-groq/1.0 (+https://github.com/dp1x/handy-groq)")
                .build()
                .expect("failed to create HTTP client"),
        }
    }
}

impl GroqClient {
    /// Transcribe audio by sending it to Groq's Whisper API.
    ///
    /// - `api_key`: the user's Groq API key
    /// - `model`: e.g. `"whisper-large-v3-turbo"`
    /// - `language`: ISO-639-1 code (e.g. `"en"`) or `"auto"`
    /// - `wav_bytes`: a complete 16 kHz mono WAV file
    pub async fn transcribe(
        &self,
        api_key: &str,
        model: &str,
        language: &str,
        wav_bytes: &[u8],
    ) -> Result<String, GroqError> {
        if api_key.is_empty() {
            return Err(GroqError::NoApiKey);
        }

        debug!(
            "Sending {} bytes to Groq (model={}, lang={})",
            wav_bytes.len(),
            model,
            language
        );

        // Build multipart form — OpenAI-compatible API.
        let mut form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(wav_bytes.to_vec())
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| GroqError::Network(format!("MIME error: {}", e)))?,
            );

        // Only send language if it's not "auto" — Groq auto-detects then.
        if language != "auto" && !language.is_empty() {
            form = form.text("language", language.to_string());
        }

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| GroqError::Network(e.to_string()))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        match status.as_u16() {
            200 => {
                let parsed: GroqResponse =
                    serde_json::from_str(&body).map_err(|_| GroqError::ParseError)?;
                debug!("Groq response: {} chars", parsed.text.len());
                Ok(parsed.text)
            }
            401 => Err(GroqError::InvalidApiKey),
            429 => Err(GroqError::RateLimited),
            _ => Err(GroqError::Network(format!(
                "HTTP {}: {}",
                status,
                body.chars().take(500).collect::<String>()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_error_display() {
        assert_eq!(
            GroqError::NoApiKey.to_string(),
            "No Groq API key set. Paste one in Settings."
        );
        assert_eq!(
            GroqError::InvalidApiKey.to_string(),
            "Groq API key is invalid. Check it in Settings."
        );
        assert_eq!(
            GroqError::RateLimited.to_string(),
            "Rate limited by Groq. Wait a moment and try again."
        );
        assert_eq!(
            GroqError::ParseError.to_string(),
            "Could not parse Groq response."
        );
    }

    #[test]
    fn test_empty_api_key_returns_error() {
        let client = GroqClient::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.transcribe("", "whisper-large-v3-turbo", "auto", &[]));
        assert_eq!(result.unwrap_err(), GroqError::NoApiKey);
    }
}
