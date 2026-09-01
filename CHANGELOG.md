# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-09-01
### Added
- Multi-city server `crates/urbania-server` (Axum 0.7 + SQLite via `persistence` crate): `GET /health`, `GET/POST /cities`, `GET/DELETE /cities/:id`, `POST /cities/:id/command`, `POST /cities/:id/save`, `WS /cities/:id/ws` with `InitialSnapshot` + `WorldDelta` (spec 43).
- `crates/shared-protocol` extended: `CityId`, `CityMeta`, `CreateCityRequest`, `WorldDelta`, `ChunkDto`, `ClientMessage`/`ServerMessage` WS envelopes, validation trait (spec 54).
- `crates/persistence` multi-city schema: `cities` + `city_chunks` (sparse delta, spec 7.2) + `historical_metrics`, `init_db`, `create_city`, `save_city_chunks` with upsert.
- `crates/transport` deterministic `RoadGraph {Node,Edge}` with snap, `validate_build`/`apply_build`, `recompute_junctions`, `generate_grid` + invariants (spec 9).
- `start.sh` now runs `urbania-server` on `PORT=8001` in background and Vite on `0.0.0.0:8000` with proxy for `/health`/`/cities`; updated `vite.config.ts` proxy.
- Updated `README.md` with new architecture, API quick test, and single-script flow.

## [0.2.0] - 2026-09-02
### Added
- WebAssembly terrain generation via `sim-core` (Perlin noise height map).
- Road graph placeholder structs and WASM bindings.
- `start.sh` auto‑installs `wasm-pack` (via cargo) and builds the WASM module before launching the dev server.
- Updated README with prerequisites and detailed start‑script flow.
- Updated roadmap to mark Phase 0 completed and Phase 1 in progress.
- Added placeholder JS stub for `sim_core` to allow development without a Rust toolchain.

## [0.1.0] - 2026-09-01
### Added
- Initial project scaffolding: directory layout, documentation files, and agent SOP.

## [0.1.0] - 2026-09-01
### Added
- Initial project scaffolding: directory layout, documentation files, and agent SOP.
