// audio/transcription/gemini_provider.rs
//
// Gemini-based transcription provider that sends audio to Google's
// Gemini API for speech-to-text. Requires a GEMINI_API_KEY.

use async_trait::async_trait;
use base64::Engine as _;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use super::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};

pub struct GeminiTranscriptionProvider {
    api_key: String,
    model: Arc<RwLock<Option<String>>>,
}

impl GeminiTranscriptionProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: Arc::new(RwLock::new(Some("gemini-2.5-flash".to_string()))),
        }
    }
}

/// Encode f32 PCM samples (16kHz mono) to a WAV byte buffer.
fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len();
    let byte_rate = sample_rate * 2; // 16-bit mono
    let data_size = (num_samples * 2) as u32;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(file_size as usize + 8);
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
                                                 // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let val = (clamped * 32767.0) as i16;
        buf.extend_from_slice(&val.to_le_bytes());
    }
    buf
}

#[async_trait]
impl TranscriptionProvider for GeminiTranscriptionProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError> {
        if audio.is_empty() {
            return Err(TranscriptionError::AudioTooShort {
                samples: 0,
                minimum: 1600,
            });
        }

        let wav_bytes = encode_wav(&audio, 16000);
        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

        let lang_hint = language
            .map(|l| format!(" The audio is in {}.", l))
            .unwrap_or_default();

        let body = serde_json::json!({
            "contents": [{
                "parts": [
                    {
                        "inline_data": {
                            "mime_type": "audio/wav",
                            "data": audio_b64
                        }
                    },
                    {
                        "text": format!(
                            "Transcribe this audio accurately. Return ONLY the transcribed text, nothing else.{}",
                            lang_hint
                        )
                    }
                ]
            }],
            "generationConfig": {
                "temperature": 0.0
            }
        });

        let model_name = self
            .model
            .read()
            .await
            .clone()
            .unwrap_or_else(|| "gemini-2.5-flash".to_string());
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model_name, self.api_key
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| {
                TranscriptionError::EngineFailed(format!("Gemini request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let err_body = response.text().await.unwrap_or_default();
            error!("Gemini transcription API error: {}", err_body);
            return Err(TranscriptionError::EngineFailed(format!(
                "Gemini API error: {}",
                err_body
            )));
        }

        let resp_json: serde_json::Value = response.json().await.map_err(|e| {
            TranscriptionError::EngineFailed(format!("Failed to parse Gemini response: {}", e))
        })?;

        let text = resp_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        info!("Gemini transcription complete: '{}'", &text);

        Ok(TranscriptResult {
            text,
            confidence: None,
            is_partial: false,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn get_current_model(&self) -> Option<String> {
        self.model.read().await.clone()
    }

    fn provider_name(&self) -> &'static str {
        "Gemini"
    }
}
