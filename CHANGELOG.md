# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0] - 2026-09-01
### Added
- `world-gen` deterministic terrain: `CHUNK_SIZE=16`, `generate_heights`/`generate_chunk` perlin (scale 0.1, seed+cx/cy offset, range [0,1], deterministic + adjacent continuity tests).
- `urbania-server` chunk streaming: `GET /cities/:id/chunks/:cx/:cy` (procedural fallback via `world-gen` + in-mem check, sparse delta spec 7.2), WS `RequestChunk` now generates procedurally, `vite.config` proxy `ws:true`.
- `IsoMap.tsx` terrain rendering: fetches 5×5 chunks around origin, draws 16×16 diamonds per chunk with `heightToColor` (water<0.30 blue, sand, grass, rock>0.65, snow>0.80), faint grid, pan with shift+wheel/middle-drag, legend bar, chunk count.

## [0.4.0] - 2026-09-01
### Added
- RoadGraph persisted per city (`city_road_graph` table) + `save_road_graph`/`load_road_graph` in `persistence` crate.
- `shared-protocol` road DTOs: `WorldPos`, `JunctionType`, `RoadNodeDto`, `RoadEdgeDto`, `RoadGraphDto`, `BuildRoadRequest`; extended `InitialSnapshot` with `road_graph` and `WorldDelta.changed_roads`.
- `transport` now wraps `RoadGraphDto` (uses `shared-protocol` types) with `from_dto`/`to_dto`, snap/validate/invariants, deterministic grid.
- `urbania-server` roads integration: `WorldWrapper.roads: RoadGraph`, `GET/POST /cities/:id/roads`, WS `Snapshot` now includes roads + `Delta.changed_roads`, `command_handler` and WS `Command(BuildRoad)` apply via `RoadGraph` and persist.
- Frontend 2.5D isometric map `apps/web/src/IsoMap.tsx` (64×32 diamond tiles, 20×20 grid, pan, click-drag road preview, auto-snap, `fetch /cities/:id/roads` + WS live sync).
- `apps/web/src/App.tsx` city selector + create city + ribbon (Build/Zone/Services/Transit/Utilities/Policies per spec 31-32) + toggle Iso/3D views.

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
