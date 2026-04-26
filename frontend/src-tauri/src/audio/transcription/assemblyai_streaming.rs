// audio/transcription/assemblyai_streaming.rs
//
// AssemblyAI Universal-Streaming integration for low-latency partial transcripts.

use super::worker::TranscriptUpdate;
use crate::audio::recording_state::AudioChunk;
use crate::database::repositories::setting::SettingsRepository;
use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;

static ASSEMBLYAI_SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(0);

const ASSEMBLYAI_HOST: &str = "wss://streaming.assemblyai.com/v3/ws";
const DEFAULT_ASSEMBLYAI_MODEL: &str = crate::config::DEFAULT_ASSEMBLYAI_MODEL;

#[derive(Debug, Clone, Serialize)]
pub struct PartialTranscriptUpdate {
    pub text: String,
    pub timestamp: String,
    pub source: String,
    pub turn_order: u64,
    pub is_final: bool,
    pub confidence: f32,
    pub audio_start_time: f64,
    pub audio_end_time: f64,
    pub duration: f64,
}

#[derive(Debug, Deserialize)]
struct AssemblyAiMessage {
    #[serde(rename = "type")]
    message_type: String,
    transcript: Option<String>,
    turn_order: Option<u64>,
    end_of_turn: Option<bool>,
    turn_is_formatted: Option<bool>,
    end_of_turn_confidence: Option<f32>,
    words: Option<Vec<AssemblyAiWord>>,
}

#[derive(Debug, Deserialize)]
struct AssemblyAiWord {
    start: Option<f64>,
    end: Option<f64>,
    confidence: Option<f32>,
}

pub fn is_assemblyai_provider(provider: &str) -> bool {
    matches!(provider, "assemblyAI" | "assemblyai" | "assembly_ai")
}

pub fn is_supported_model(model: &str) -> bool {
    matches!(
        model,
        "universal-streaming-english"
            | "universal-streaming-multilingual"
            | "whisper-rt"
            | "u3-rt-pro"
    )
}

pub async fn resolve_api_key<R: Runtime>(app: &AppHandle<R>) -> Result<String> {
    let state = app.state::<crate::state::AppState>();
    let pool = state.db_manager.pool();

    let db_key = SettingsRepository::get_transcript_api_key(pool, "assemblyAI")
        .await
        .context("Failed to load AssemblyAI API key from settings")?;

    if let Some(key) = db_key.filter(|key| !key.trim().is_empty()) {
        return Ok(key);
    }

    if let Some(key) = env_key("ASSEMBLY_AI_API_KEY") {
        return Ok(key);
    }

    Err(anyhow!(
        "AssemblyAI API key not found. Set ASSEMBLY_AI_API_KEY in .env or save a key in transcription settings."
    ))
}

pub fn start_assemblyai_streaming_task<R: Runtime>(
    app: AppHandle<R>,
    audio_receiver: mpsc::UnboundedReceiver<AudioChunk>,
    api_key: String,
    model: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(error) =
            run_assemblyai_streaming(app.clone(), audio_receiver, api_key, model).await
        {
            error!("AssemblyAI streaming transcription failed: {}", error);
            let _ = app.emit(
                "transcription-error",
                serde_json::json!({
                    "error": error.to_string(),
                    "userMessage": format!("AssemblyAI transcription failed: {}", error),
                    "actionable": true
                }),
            );
        }

        let _ = app.emit("transcription-complete", ());
    })
}

async fn run_assemblyai_streaming<R: Runtime>(
    app: AppHandle<R>,
    mut audio_receiver: mpsc::UnboundedReceiver<AudioChunk>,
    api_key: String,
    model: String,
) -> Result<()> {
    let model = if is_supported_model(&model) {
        model
    } else {
        DEFAULT_ASSEMBLYAI_MODEL.to_string()
    };

    let format_turns = if model == "u3-rt-pro" {
        ""
    } else {
        "&format_turns=true"
    };
    let endpoint = format!(
        "{}?sample_rate=16000&encoding=pcm_s16le{}&speech_model={}",
        ASSEMBLYAI_HOST,
        format_turns,
        urlencoding::encode(&model)
    );

    let mut request = endpoint
        .into_client_request()
        .context("Failed to build AssemblyAI WebSocket request")?;
    request.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(api_key.trim()).context("Invalid AssemblyAI API key header")?,
    );

    info!(
        "Connecting to AssemblyAI streaming transcription ({})",
        model
    );
    let (ws_stream, _) = connect_async(request)
        .await
        .context("Failed to connect to AssemblyAI streaming API")?;
    let (mut write, mut read) = ws_stream.split();

    let sender_handle = tokio::spawn(async move {
        while let Some(chunk) = audio_receiver.recv().await {
            let pcm_bytes = audio_chunk_to_pcm16(chunk);
            if pcm_bytes.is_empty() {
                continue;
            }

            if let Err(error) = write.send(Message::Binary(pcm_bytes.into())).await {
                warn!("Failed to send audio to AssemblyAI: {}", error);
                return;
            }
        }

        let terminate = serde_json::json!({ "type": "Terminate" }).to_string();
        let _ = write.send(Message::Text(terminate.into())).await;
    });

    let mut finalized_turns = HashSet::new();
    let mut speech_detected_emitted = false;

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                handle_assemblyai_message(
                    &app,
                    &text,
                    &mut finalized_turns,
                    &mut speech_detected_emitted,
                )?;
            }
            Ok(Message::Close(frame)) => {
                info!("AssemblyAI WebSocket closed: {:?}", frame);
                break;
            }
            Ok(_) => {}
            Err(error) => {
                return Err(anyhow!("AssemblyAI WebSocket read failed: {}", error));
            }
        }
    }

    let _ = sender_handle.await;
    Ok(())
}

fn handle_assemblyai_message<R: Runtime>(
    app: &AppHandle<R>,
    text: &str,
    finalized_turns: &mut HashSet<u64>,
    speech_detected_emitted: &mut bool,
) -> Result<()> {
    let message: AssemblyAiMessage =
        serde_json::from_str(text).context("Failed to parse AssemblyAI message")?;

    match message.message_type.as_str() {
        "Begin" => {
            info!("AssemblyAI streaming session started");
            Ok(())
        }
        "Turn" => {
            let transcript = message
                .transcript
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string();
            if transcript.is_empty() {
                return Ok(());
            }

            if !*speech_detected_emitted {
                *speech_detected_emitted = true;
                let _ = app.emit(
                    "speech-detected",
                    serde_json::json!({ "message": "Speech activity detected" }),
                );
            }

            let turn_order = message.turn_order.unwrap_or(0);
            let (audio_start_time, audio_end_time, confidence) = turn_metadata(&message);
            let duration = (audio_end_time - audio_start_time).max(0.0);
            let is_final =
                message.end_of_turn.unwrap_or(false) && message.turn_is_formatted.unwrap_or(true);

            if is_final {
                if finalized_turns.contains(&turn_order) {
                    return Ok(());
                }
                finalized_turns.insert(turn_order);

                let sequence_id = ASSEMBLYAI_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst);
                let update = TranscriptUpdate {
                    text: transcript,
                    timestamp: format_current_timestamp(),
                    source: "AssemblyAI".to_string(),
                    sequence_id,
                    chunk_start_time: audio_start_time,
                    is_partial: false,
                    confidence,
                    audio_start_time,
                    audio_end_time,
                    duration,
                };

                app.emit("transcript-update", &update)
                    .context("Failed to emit AssemblyAI final transcript")?;
            } else {
                let update = PartialTranscriptUpdate {
                    text: transcript,
                    timestamp: format_current_timestamp(),
                    source: "AssemblyAI".to_string(),
                    turn_order,
                    is_final: false,
                    confidence,
                    audio_start_time,
                    audio_end_time,
                    duration,
                };

                app.emit("transcript-partial-update", &update)
                    .context("Failed to emit AssemblyAI partial transcript")?;
            }

            Ok(())
        }
        "Termination" => {
            info!("AssemblyAI streaming session terminated");
            Ok(())
        }
        other => {
            info!("Ignoring AssemblyAI message type: {}", other);
            Ok(())
        }
    }
}

fn audio_chunk_to_pcm16(chunk: AudioChunk) -> Vec<u8> {
    let samples_16k = if chunk.sample_rate == 16000 {
        chunk.data
    } else {
        crate::audio::audio_processing::resample_audio(&chunk.data, chunk.sample_rate, 16000)
    };

    let mut bytes = Vec::with_capacity(samples_16k.len() * 2);
    for sample in samples_16k {
        let sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    bytes
}

fn turn_metadata(message: &AssemblyAiMessage) -> (f64, f64, f32) {
    let words = message.words.as_deref().unwrap_or(&[]);

    let audio_start_time = words
        .first()
        .and_then(|word| word.start)
        .map(assembly_time_to_seconds)
        .unwrap_or(0.0);

    let audio_end_time = words
        .last()
        .and_then(|word| word.end)
        .map(assembly_time_to_seconds)
        .unwrap_or(audio_start_time);

    let confidences: Vec<f32> = words.iter().filter_map(|word| word.confidence).collect();
    let confidence = if confidences.is_empty() {
        message.end_of_turn_confidence.unwrap_or(0.85)
    } else {
        confidences.iter().sum::<f32>() / confidences.len() as f32
    };

    (audio_start_time, audio_end_time, confidence)
}

fn assembly_time_to_seconds(value: f64) -> f64 {
    // AssemblyAI streaming word offsets are millisecond-based.
    value / 1000.0
}

fn format_current_timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

fn env_key(name: &str) -> Option<String> {
    if let Ok(value) = std::env::var(name) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }

    for path in env_file_candidates() {
        if let Some(value) = read_env_key(&path, name) {
            return Some(value);
        }
    }

    None
}

fn env_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(".env"));
        if let Some(parent) = current_dir.parent() {
            candidates.push(parent.join(".env"));
            if let Some(grandparent) = parent.parent() {
                candidates.push(grandparent.join(".env"));
            }
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join(".env"));
    if let Some(parent) = manifest_dir.parent() {
        candidates.push(parent.join(".env"));
        if let Some(grandparent) = parent.parent() {
            candidates.push(grandparent.join(".env"));
        }
    }

    candidates
}

fn read_env_key(path: &PathBuf, name: &str) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim() == name {
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}
