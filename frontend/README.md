# Friday - Frontend

A modern desktop application for recording, transcribing, and analyzing meetings with AI assistance. Built with Next.js and Tauri for a native desktop experience.

## Features

- Real-time audio recording from both microphone and system audio
- Live transcription using Whisper ASR (locally running)
- Native desktop integration using Tauri
- Speaker diarization support
- Rich text editor for note-taking
- Privacy-focused: All processing happens locally

## Prerequisites

### For macOS:
- Node.js (v18 or later)
- Rust (latest stable)
- pnpm (v8 or later)
- [Xcode Command Line Tools](https://developer.apple.com/download/all/?q=xcode)

### For Windows:
- Node.js (v18 or later)
- Rust (latest stable)
- pnpm (v8 or later)
- Visual Studio Build Tools with C++ development tools
- Windows 10 or later


## Project Structure

```
/frontend
├── src/                   # Next.js frontend code
├── src-tauri/             # Rust backend for Tauri
├── whisper-server-package/ # Local transcription server
│   ├── models/            # Whisper models
│   ├── whisper-server     # Pre-built server binary
│   └── run-server.sh      # Script to start the server
├── public/                # Static assets
└── package.json           # Project dependencies
```

## Installation

### For macOS:

1. Install prerequisites:
   ```bash
   # Install Homebrew if not already installed
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   
   # Install Node.js
   brew install node
   
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install pnpm
   npm install -g pnpm
   
   # Install Xcode Command Line Tools
   xcode-select --install
   ```

2. Clone the repository and navigate to the frontend directory:
   ```bash
   git clone https://github.com/Zackriya-Solutions/meeting-minutes
   cd meeting-minutes
   git submodule update --init --recursive
   cd frontend
   ```
  

3. Install dependencies:
   ```bash
   pnpm install
   ```

### For Windows:

1. Install prerequisites:
   - Install [Node.js](https://nodejs.org/) (v18 or later)
   - Install [Rust](https://www.rust-lang.org/tools/install)
   - Install pnpm: `npm install -g pnpm`
   - Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with C++ development tools

2. Clone the repository and navigate to the frontend directory:
   ```cmd
   git clone https://github.com/Zackriya-Solutions/meeting-minutes
   cd meeting-minutes
   git submodule update --init --recursive
   cd frontend
   ```

3. Install dependencies:
   ```cmd
   pnpm install
   ```

## Running the App

### For macOS:

Use either of these development entrypoints:
```bash
pnpm run tauri:dev
# or
./dev-gpu.sh
```

To build a production version:
```bash
pnpm run tauri:build
# or
./build-gpu.sh
```

### For Windows:

Use either of these development entrypoints:
```cmd
pnpm run tauri:dev
REM or
dev-gpu.bat
```

To build a production version:
```cmd
pnpm run tauri:build
REM or
build-gpu.bat
```

## Whisper Transcription Server

The application includes a pre-built Whisper server for real-time speech recognition:

- Located in `whisper-server-package/`
- Supports speaker diarization
- Runs locally for privacy
- Uses Metal acceleration on macOS

To run the Whisper server manually:
```bash
cd whisper-server-package
./run-server.sh
```

The server will be available at http://localhost:8178

## Development

### Frontend (Next.js)
- The frontend is built with Next.js and Tailwind CSS
- Source code is in the `src/` directory
- To run only the frontend: `pnpm run dev`

### Backend (Tauri)
- The Rust backend is in the `src-tauri/` directory
- Handles audio capture, file system access, and native integrations
- To run the full desktop app: `pnpm run tauri:dev`

## Troubleshooting

### Common Issues on macOS
- If you encounter permission issues with scripts, make them executable:
  ```bash
  chmod +x dev-gpu.sh build-gpu.sh clean_run.sh clean_build.sh whisper-server-package/run-server.sh
  ```
- If this is a fresh clone and native builds fail, rerun:
  ```bash
  git submodule update --init --recursive
  ```
- For microphone access issues, ensure the app has microphone permissions in System Preferences
- If the Whisper server fails to start, check if port 8178 is already in use

### Common Issues on Windows
- If you encounter build errors, ensure Visual Studio Build Tools are properly installed
- For audio capture issues, check Windows privacy settings for microphone access
- If the app fails to start, try running Command Prompt as administrator

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
