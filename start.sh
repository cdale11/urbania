#!/usr/bin/env bash
set -e

# Activate the `urbania` Conda environment if conda is available
if command -v conda >/dev/null 2>&1; then
  eval "$(conda shell.bash hook)"
  conda activate urbania || true
fi

ROOT="$(cd "$(dirname "$0")" && pwd)"
WEB_DIR="$ROOT/apps/web"

echo "==> Installing frontend dependencies"
cd "$WEB_DIR"
if [ -f package-lock.json ]; then
  npm ci
else
  npm install
fi

echo "==> Building WASM (sim-core -> public/pkg)"
if command -v wasm-pack >/dev/null 2>&1; then
  npm run build-wasm || echo "wasm build failed - continuing with JS fallback"
else
  echo "wasm-pack not found - attempting cargo install"
  if command -v cargo >/dev/null 2>&1; then
    cargo install wasm-pack || true
    command -v wasm-pack >/dev/null 2>&1 && npm run build-wasm || echo "wasm-pack unavailable - skipping"
  else
    echo "cargo not found - skipping wasm build"
  fi
fi

cd "$ROOT"
echo "==> Starting urbania-server (multi-city, SQLite) on 0.0.0.0:8001"
# Build server first for faster startup log
if command -v cargo >/dev/null 2>&1; then
  # Use PORT env to avoid conflict with vite on 8000
  PORT=8001 cargo run -p urbania-server --release &
  SERVER_PID=$!
  echo "    server pid $SERVER_PID"
  # Ensure server is killed on exit
  trap "kill $SERVER_PID 2>/dev/null || true" EXIT INT TERM
  # Wait for health endpoint (max 15s)
  for i in $(seq 1 15); do
    if curl -sf http://localhost:8001/health >/dev/null 2>&1; then
      echo "    server ready"
      break
    fi
    sleep 1
  done
else
  echo "cargo not found - cannot start server"
  SERVER_PID=""
fi

echo "==> Starting Vite frontend on 0.0.0.0:8000 (proxying /health,/cities to server 8001)"
cd "$WEB_DIR"
npm run dev
