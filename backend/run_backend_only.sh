#!/bin/bash
# Run only the FastAPI backend (no Whisper server). Use for testing Friday /friday-extract.
# From repo root:  ./backend/run_backend_only.sh
# Or from backend: ./run_backend_only.sh

set -e
cd "$(dirname "$0")"

if [ ! -f ".env" ]; then
  echo "No .env found. Copying from temp.env..."
  cp temp.env .env
  echo "Edit backend/.env and set GEMINI_API_KEY (and other keys if needed), then run this again."
  exit 1
fi

if [ ! -d "venv" ]; then
  echo "No venv found. Create one with: python3.12 -m venv venv && source venv/bin/activate && pip install -r requirements.txt"
  exit 1
fi

echo "Starting backend on http://127.0.0.1:5167"
echo "API docs: http://127.0.0.1:5167/docs"
echo "Ctrl+C to stop."
source venv/bin/activate
python app/main.py
