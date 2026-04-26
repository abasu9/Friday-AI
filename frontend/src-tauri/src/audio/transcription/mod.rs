// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.

pub mod assemblyai_streaming;
pub mod engine;
pub mod gemini_provider;
pub mod parakeet_provider;
pub mod provider;
pub mod whisper_provider;
pub mod worker;

// Re-export commonly used types
pub use assemblyai_streaming::{
    is_assemblyai_provider, resolve_api_key as resolve_assemblyai_api_key,
    start_assemblyai_streaming_task, PartialTranscriptUpdate,
};
pub use engine::{
    get_or_init_transcription_engine, get_or_init_whisper, validate_transcription_model_ready,
    TranscriptionEngine,
};
pub use gemini_provider::GeminiTranscriptionProvider;
pub use parakeet_provider::ParakeetProvider;
pub use provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
pub use whisper_provider::WhisperProvider;
pub use worker::{reset_speech_detected_flag, start_transcription_task, TranscriptUpdate};
