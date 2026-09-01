# Urbania

A browser‑native city simulation based on the *Browser City Builder* implementation specification. This repository contains the full stack: Rust simulation core, TypeScript/React UI, Three.js rendering, and supporting tooling.

## Quick start

```bash
# Ensure the `urbania` Conda environment is installed and activated
conda activate urbania

# Run the single start script (starts the front‑end dev server on 0.0.0.0:8000)
./start.sh
```

The Vite dev server will be reachable at `http://0.0.0.0:8000`.

## Development

- **Front‑end** – Located in `apps/web`. It is a Vite + React + TypeScript project using the `@vitejs/plugin-react` plugin. The dev server is configured to listen on `0.0.0.0` port `8000`.
- **Simulation core** – The Rust `sim-core` crate under `crates/sim-core` provides a deterministic RNG, a fixed‑step simulation clock, and basic scaffolding for future subsystems.
- **Workspace** – The top‑level `Cargo.toml` defines a workspace containing all crates under `crates/*`.
- **Single‑script launch** – `start.sh` activates the `urbania` Conda environment, installs front‑end dependencies (if needed), and runs `npm run dev`.

## Project structure

- `apps/web` – Vite + React UI.
- `crates/*` – Rust crates for simulation, generation, rendering, etc.
- `tools/*` – Helper tools (map generator, replay runner, benchmarks, etc.).
- `packages/*` – Shared UI components and protocol definitions.
- `AGENTS.md` – Agent SOP and guidelines.
- `roadmap.md` – Development phases and priorities.
- `CHANGELOG.md` – Project changelog.
- `mistakes.md` – Common pitfalls.
