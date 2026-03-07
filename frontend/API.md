# Whisper.cpp Live Transcription API Documentation

## Overview

The Whisper.cpp Live Transcription API provides real-time speech-to-text transcription using OpenAI's Whisper model. This API supports streaming audio input and returns timestamped transcriptions as they become available.

## Server Configuration

### Starting the Server

```bash
./bin/whisper-server [options] -m [model_path]
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --model` | Path to the Whisper model file | Required |
| `-h, --host` | Host to bind the server | "127.0.0.1" |
| `-p, --port` | Port to bind the server | 8178 |
| `-t, --threads` | Number of threads to use | 4 |
| `-c, --context` | Maximum context size | 16384 |
| `-l, --language` | Language to use for transcription | "auto" |
| `-tr, --translate` | Translate to English | false |
| `-ps, --print-special` | Print special tokens | false |
| `-pc, --print-colors` | Print colors | false |

## API Endpoints

### Live Transcription

**Endpoint**: `/stream`  
**Method**: POST  
**Content-Type**: multipart/form-data

Streams audio data for real-time transcription.

#### Request Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `audio` | Binary | Raw audio data in 32-bit float PCM format |

#### Audio Requirements

- Sample Rate: 16000 Hz
- Format: 32-bit float PCM
- Minimum Length: 1000ms (16000 samples)
- Recommended Chunk Size: 500ms (8000 samples)

#### Response Format

```json
{
    "segments": [
        {
            "text": "Transcribed text segment",
            "t0": 0.0,    // Start time in seconds
            "t1": 1.0     // End time in seconds
        }
    ],
    "buffer_size_ms": 1200  // Current buffer size in milliseconds
}
```

#### Error Response

```json
{
    "error": "Error message description"
}
```

### Example Usage

#### JavaScript Example
```javascript
// Initialize audio capture
const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
const audioContext = new AudioContext({ sampleRate: 16000 });
const source = audioContext.createMediaStreamSource(stream);
const processor = audioContext.createScriptProcessor(4096, 1, 1);

// Send audio chunks
async function sendAudioChunk(audioData) {
    const formData = new FormData();
    formData.append('audio', new Blob([audioData], { 
        type: 'application/octet-stream' 
    }));

    const response = await fetch('/stream', {
        method: 'POST',
        body: formData
    });

    const result = await response.json();
    // Handle transcription results
    if (result.segments) {
        result.segments.forEach(segment => {
            console.log(`[${segment.t0}s - ${segment.t1}s]: ${segment.text}`);
        });
    }
}
```

#### Rust Example
```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Segment {
    text: String,
    t0: f32,
    t1: f32,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    segments: Vec<Segment>,
    buffer_size_ms: u32,
}

struct AudioStreamer {
    client: reqwest::Client,
    sender: mpsc::Sender<Vec<f32>>,
    receiver: mpsc::Receiver<Vec<f32>>,
}

impl AudioStreamer {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            client: reqwest::Client::new(),
            sender,
            receiver,
        }
    }

    async fn start_recording(&self) -> Result<(), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or("No input device available")?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16000),
            buffer_size: cpal::BufferSize::Fixed(4096),
        };

        let sender = self.sender.clone();
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &_| {
                sender.send(data.to_vec()).unwrap();
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(())
    }

    async fn process_audio(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = Vec::new();
        const CHUNK_INTERVAL: Duration = Duration::from_millis(500);
        let mut last_send = std::time::Instant::now();

        while let Ok(chunk) = self.receiver.try_recv() {
            buffer.extend(chunk);

            if last_send.elapsed() >= CHUNK_INTERVAL {
                // Create multipart form
                let form = multipart::Form::new()
                    .part("audio", multipart::Part::bytes(
                        buffer.iter()
                            .flat_map(|&x| x.to_le_bytes().to_vec())
                            .collect::<Vec<u8>>()
                    ).mime_str("application/octet-stream")?);

                // Send request
                let response = self.client
                    .post("http://localhost:8178/stream")
                    .multipart(form)
                    .send()
                    .await?;

                // Handle response
                if let Ok(result) = response.json::<TranscriptionResponse>().await {
                    for segment in result.segments {
                        println!("[{:.2}s - {:.2}s]: {}", 
                            segment.t0, segment.t1, segment.text);
                    }
                }

                buffer.clear();
                last_send = std::time::Instant::now();
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let streamer = AudioStreamer::new();
    
    // Start recording
    streamer.start_recording().await?;
    
    // Process audio in parallel
    let process_handle = tokio::spawn(async move {
        streamer.process_audio().await
    });

    // Wait for user input to stop
    let mut input = String::new();
    println!("Press Enter to stop recording...");
    std::io::stdin().read_line(&mut input)?;

    process_handle.abort();
    Ok(())
}
```

#### Dependencies (Cargo.toml)
```toml
[dependencies]
cpal = "0.15"
reqwest = { version = "0.11", features = ["multipart", "json"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Server Implementation Details

### Audio Processing

1. **Buffer Management**
   - Server maintains a rolling buffer of audio samples
   - Minimum 1000ms of audio required for processing
   - 200ms overlap between consecutive chunks for context

2. **Transcription Process**
   - Audio is processed using Whisper model
   - Results include text and precise timestamps
   - Server maintains context between chunks

### Performance Considerations

1. **Memory Usage**
   - Audio buffer size is limited to processing window
   - Old audio data is automatically cleared
   - Overlap window maintains transcription context

2. **Threading**
   - Server uses mutex for thread-safe model access
   - Multiple requests can be handled concurrently
   - Processing is done in separate threads

## Best Practices

1. **Audio Capture**
   - Use correct sample rate (16000 Hz)
   - Send regular chunks (recommended: 500ms)
   - Maintain consistent audio stream

2. **Error Handling**
   - Implement reconnection logic
   - Handle network interruptions gracefully
   - Monitor buffer status

3. **UI Implementation**
   - Show real-time feedback
   - Display buffer status
   - Implement proper error messages

## Limitations

1. **Audio Requirements**
   - Minimum 1000ms of audio needed
   - Fixed sample rate of 16000 Hz
   - Single channel audio only

2. **Processing**
   - Processing time depends on model size
   - Some latency in real-time transcription
   - Memory usage scales with audio length

## Security Considerations

1. **Rate Limiting**
   - Implement appropriate rate limiting
   - Monitor resource usage
   - Handle concurrent connections

2. **Input Validation**
   - Validate audio format
   - Check content length
   - Sanitize all inputs

## Example Implementation

See `examples/server/public/index.html` for a complete frontend implementation example.

## Support

For issues and feature requests, please refer to the GitHub repository:
[whisper.cpp](https://github.com/ggerganov/whisper.cpp)
