# Urbania

A browser‑native city simulation based on the *Browser City Builder* implementation specification. Full simulation, multi-city server, 2.5D isometric client, deterministic Rust core.

## Quick start

```bash
conda activate urbania
./start.sh
```

* `urbania-server` (Rust/Axum + SQLite) listens on `0.0.0.0:8001` — `GET /health`, `GET/POST /cities`, `WS /cities/:id/ws`
* Vite + React frontend on `0.0.0.0:8000` (proxies `/health` and `/cities` to the server)
* Open `http://localhost:8000` — create a city, then connect via WebSocket for live deltas

`start.sh` will:

1. Activate the `urbania` Conda environment
2. `npm ci` in `apps/web`
3. Build `sim-core` WASM to `apps/web/public/pkg` (skips if `wasm-pack` unavailable)
4. `cargo run -p urbania-server` on `PORT=8001` in background (SQLite `urbania.db`)
5. Launch Vite dev server on `0.0.0.0:8000`

### Prerequisites

- Rust stable + `cargo`, `wasm-pack` (auto-installed), Node 20 + npm
- Conda env `urbania` (optional but recommended)

## Architecture (spec: `city_skylines_clone_implementation_spec.md`)

- **Server** `crates/urbania-server` — Axum, SQLite (`persistence` crate), deterministic `sim-core`, `transport::RoadGraph` (Roads & Traffic first)
- **Protocol** `crates/shared-protocol` — `CityId`, `CityMeta`, `PlayerCommand`, `WorldDelta`, `ClientMessage`/`ServerMessage` (snapshot + deltas, spec 43)
- **Persistence** `crates/persistence` — multi-city `cities` + `city_chunks` tables, sparse delta storage (spec 7.2, 40)
- **Transport** `crates/transport` — deterministic `RoadGraph {Node, Edge}` with snap/validate/apply (spec 9)
- **Client** `apps/web` — Vite + React + Three.js (WebGPU/WebGL2 path per spec 3.2), isometric 2.5D map
- **Determinism** — `DeterministicRng` streams, `SimSystem::cadence` scheduler (spec 5-6)

## Project structure

- `apps/web` — frontend
- `crates/*` — `sim-core`, `world-gen`, `render-data`, `procgen`, `economy`, `transport`, `services`, `agents`, `ml-runtime`, `persistence`, `shared-protocol`, `urbania-server`
- `tools/*`, `packages/*`, `AGENTS.md`, `roadmap.md`, `CHANGELOG.md`, `mistakes.md`

## API quick test

```bash
curl http://localhost:8001/health
curl http://localhost:8001/cities
curl -X POST http://localhost:8001/cities -H 'Content-Type: application/json' -d '{"name":"Alpha"}'
# WS: ws://localhost:8001/cities/1/ws  -> receives ServerMessage::Snapshot then deltas
```
