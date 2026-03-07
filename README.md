<h1>Friday</h1>
<p>
    Privacy-first AI meeting assistant for local capture, live transcription, and meeting summaries. Friday is a native desktop app that records and summarizes meetings without shipping your raw audio or transcripts to the cloud. The product is built on a privacy-first local-first architecture with a Tauri desktop shell, Rust audio/transcription services, and a Next.js interface. 
  
  Website Link : [tinyurl.com/2nvsk8ek](https://tinyurl.com/2nvsk8ek)
  
  Demo Link : [youtu.be/QXsBkFJ8ack](https://youtu.be/QXsBkFJ8ack)

  </p>
</div>


<div align="center">
  <img src="docs/FRIDAY_logo.jpg" width="1250" alt="Friday logo" />
</div>

---

## Table of Contents


- [What Friday is for](#what-friday-is-for)
- [Core capabilities](#core-capabilities)
- [Project structure](#project-structure)
- [Architecture overview](#architecture-overview)
- [Requirements](#requirements)
- [Running Friday in development](#running-friday-in-development)
- [Building from source](#building-from-source)
- [Repository links](#repository-links)
- [Contributing](#contributing)
- [License](#license)

## What Friday is for

Friday is meant for teams and professionals who need accurate meeting notes while retaining full control of their data. It is designed for:

- Local-first transcription and storage
- Real-time transcript updates
- AI summaries with configurable providers
- Meeting export and editing workflows
- Stronger privacy controls than cloud-only assistants

## Core capabilities

- **Local-first by default**  
  Meetings are recorded and processed in local pipelines on your machine.
- **Professional audio mixing**  
  The audio layer synchronizes microphone and system audio with voice-activity-based processing for transcription quality.
- **Live transcripts + summaries**  
  See transcriptions as they are produced and generate meeting summaries after meetings end.
- **Cross-platform desktop app**  
  Built with Tauri for macOS, Windows, and Linux targets.
- **Flexible AI provider support**
  Gemini (primary), with support for local and external providers for summary generation.
- **Privacy-first defaults**  
  No raw audio/notes upload required for core operation.

## Project structure

- `frontend/`  
  Desktop shell, UI, and all local Rust audio/transcription glue.
- `backend/`  
  FastAPI service for meeting persistence, summarization endpoints, and deployment helpers.
- `docs/`  
  Operational and build documentation.

## Architecture overview

```text
Frontend (Next.js UI)  <->  Tauri command bridge  <->  Rust services
                                                           |
                                                           +-> Audio pipeline + recording
                                                           +-> Whisper integration
                                                           +-> Local file outputs

Frontend HTTP/WebSocket   <->  FastAPI backend  <->  SQLite + LLM providers
                                                           |
                                                           +-> Whisper server for transcription orchestration
```

## Requirements

- Rust (stable toolchain)
- Node.js 20+ and pnpm
- Python 3.8+ (for backend workflows)
- On macOS: microphone + screen recording permission for full system-audio capture
- GPU support depends on platform:
  - macOS: Metal/CoreML
  - Windows/Linux: CUDA/Vulkan where available

## Running Friday in development

1. Install frontend dependencies:

```bash
cd frontend
pnpm install
```

2. Start frontend in Tauri mode:

```bash
pnpm run tauri:dev
```

3. Start backend (recommended for full stack features):

```bash
cd backend
./clean_start_backend.sh   # macOS/Linux
```

Windows users can use the PowerShell workflow documented in `backend/README.md`.

Default local ports:

- Frontend dev server: `3118`
- Backend API: `5167`
- Whisper server: `8178`

4. Backend docs are available at `http://localhost:5167/docs`.

If you only need to test frontend rendering, you can run `pnpm run dev` from `frontend` for the Next.js app alone.

## Building from source

- Frontend scripts (from `frontend`):
  - `pnpm run tauri:dev` full app development mode
  - `pnpm run tauri:dev:cpu` CPU-only development build
  - `pnpm run tauri:dev:metal` Apple Silicon/Metal path
  - `pnpm run tauri:dev:cuda` CUDA path
  - `pnpm run tauri:dev:vulkan` Vulkan path
  - `pnpm run tauri:build` production app build

- Backend scripts and deployment options:
  - Docker-based and native setup instructions in `backend/README.md`
  - Whisper model options: `tiny`, `base`, `small`, `medium`, `large-v3` and more

## Repository links
- Website documentation: `tinyurl.com/2nvsk8ek`
- Architecture details: `docs/architecture.md`
- Build details: `docs/BUILDING.md`
- Linux build notes: `docs/building_in_linux.md`
- Backend setup: `backend/README.md`
- Frontend notes: `frontend/README.md`

## Contributing

Contributions are welcome.

- Keep PRs focused and scoped.
- Update docs when behavior or command flow changes.
- Use existing Rust and TypeScript patterns in the codebase.
- See `CONTRIBUTING.md` for full project process.

## License

MIT License. See `LICENSE`.

