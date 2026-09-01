#!/usr/bin/env bash
set -e

# Activate the `urbania` Conda environment if conda is available
if command -v conda >/dev/null 2>&1; then
  # Initialize conda for bash
  eval "$(conda shell.bash hook)"
  conda activate urbania
fi

# Change to the web front‑end directory
cd "$(dirname "$0")/apps/web"

# Install front‑end dependencies (use npm ci if lockfile exists, otherwise npm install)
if [ -f package-lock.json ]; then
  npm ci
else
  npm install
fi

# Run the Vite development server (listening on 0.0.0.0:8000 as configured in vite.config.ts)
npm run dev
