use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::StreamExt;
use log::{error, info};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use persistence::{
    create_city, create_zone, delete_city, delete_zone, get_city, init_db, list_cities, list_parcels,
    list_zones, load_city_state, load_road_graph, save_city_state, save_road_graph, update_city_tick,
};
use shared_protocol::{
    BuildRoadRequest, ChunkDto, CityId, CityMeta, ClientMessage, CommandResult, CreateCityRequest,
    CreateCityResponse, CreateZoneRequest, InitialSnapshot, ParcelDto, PlayerCommand, RoadGraphDto,
    ServerMessage, WorldPos, ZoneDto,
};
use sim_core::{Chunk, DeterministicRng, SimClock, SimulationState, TICK_MS};
use transport::RoadGraph;
use world_gen::generate_chunk;

// ---------------------- Config ----------------------
#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_db_path")]
    database_path: String,
}
fn default_port() -> u16 { 8000 }
fn default_db_path() -> String { "urbania.db".into() }
impl Default for Config {
    fn default() -> Self { Self { port: default_port(), database_path: default_db_path() } }
}

// ---------------------- WorldWrapper + System ----------------------
struct WorldWrapper {
    city_id: CityId,
    city_name: String,
    world: SimulationState,
    roads: RoadGraph,
    systems: Vec<Box<dyn System + Send + Sync>>,
}

impl WorldWrapper {
    fn new(city_id: CityId, city_name: String, seed: u64, tick: u64) -> Self {
        let world = SimulationState {
            seed,
            rng: DeterministicRng::from_seed(seed),
            clock: SimClock { tick, time_ms: tick * TICK_MS },
            chunks: vec![],
        };
        Self { city_id, city_name, world, roads: RoadGraph::new(), systems: vec![] }
    }
    fn from_state(city_id: CityId, city_name: String, state: SimulationState, roads: RoadGraph) -> Self {
        Self { city_id, city_name, world: state, roads, systems: vec![] }
    }
    fn register_system<S>(&mut self, s: S) where S: System + Send + Sync + 'static { self.systems.push(Box::new(s)); }
    fn tick(&mut self, dt: Duration) {
        self.world.clock.tick();
        for sys in &mut self.systems { sys.update(&mut self.world, dt); }
    }
}

pub trait System { fn update(&mut self, world: &mut SimulationState, dt: Duration); }
struct DummySystem;
impl System for DummySystem { fn update(&mut self, _: &mut SimulationState, _: Duration) {} }

// ---------------------- Shared State ----------------------
type CityMap = HashMap<CityId, Arc<Mutex<WorldWrapper>>>;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    cities: Arc<Mutex<CityMap>>,
}

async fn get_or_load_city(state: &AppState, city_id: CityId) -> Result<Arc<Mutex<WorldWrapper>>, (StatusCode, String)> {
    {
        let map = state.cities.lock().await;
        if let Some(w) = map.get(&city_id) { return Ok(Arc::clone(w)); }
    }
    let meta = get_city(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, format!("city {city_id} not found")))?;
    let sim_state = load_city_state(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .unwrap_or_else(|| SimulationState { seed: meta.seed, rng: DeterministicRng::from_seed(meta.seed), clock: SimClock { tick: meta.tick, time_ms: meta.tick * TICK_MS }, chunks: vec![] });
    // Load road graph
    let road_json = load_road_graph(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let roads = road_json.and_then(|v| serde_json::from_value::<RoadGraphDto>(v).ok()).map(RoadGraph::from_dto).unwrap_or_else(RoadGraph::new);
    let wrapper = WorldWrapper::from_state(meta.id, meta.name.clone(), sim_state, roads);
    let arc = Arc::new(Mutex::new(wrapper));
    let mut map = state.cities.lock().await;
    map.insert(city_id, Arc::clone(&arc));
    Ok(arc)
}

// ---------------------- HTTP handlers ----------------------
async fn health() -> impl IntoResponse { Json(serde_json::json!({"status":"ok"})) }

async fn list_cities_handler(State(state): State<AppState>) -> Result<Json<Vec<CityMeta>>, (StatusCode, String)> {
    let cities = list_cities(&state.db).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(cities))
}

async fn create_city_handler(State(state): State<AppState>, Json(req): Json<CreateCityRequest>) -> Result<Json<CreateCityResponse>, (StatusCode, String)> {
    if req.name.trim().is_empty() { return Err((StatusCode::BAD_REQUEST, "name required".into())); }
    let seed = req.seed.unwrap_or_else(|| {
        use rand::RngCore;
        let mut b = [0u8;8]; rand::rngs::OsRng.fill_bytes(&mut b); u64::from_le_bytes(b)
    });
    let meta = create_city(&state.db, &req.name, seed).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let wrapper = WorldWrapper::new(meta.id, meta.name.clone(), meta.seed, 0);
    let arc = Arc::new(Mutex::new(wrapper));
    state.cities.lock().await.insert(meta.id, arc);
    info!("Created city {} id={} seed={}", meta.name, meta.id, meta.seed);
    Ok(Json(CreateCityResponse { city: meta }))
}

async fn get_city_handler(State(state): State<AppState>, Path(city_id): Path<CityId>) -> Result<Json<CityMeta>, (StatusCode, String)> {
    let meta = get_city(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "city not found".into()))?;
    Ok(Json(meta))
}

async fn delete_city_handler(State(state): State<AppState>, Path(city_id): Path<CityId>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    delete_city(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    state.cities.lock().await.remove(&city_id);
    Ok(Json(serde_json::json!({"status":"deleted","id":city_id})))
}

#[derive(Debug, Deserialize)]
struct CommandBody { command: PlayerCommand }

async fn command_handler(State(state): State<AppState>, Path(city_id): Path<CityId>, Json(body): Json<CommandBody>) -> Result<Json<CommandResult>, (StatusCode, String)> {
    let wrapper = get_or_load_city(&state, city_id).await?;
    let mut guard = wrapper.lock().await;
    let result = match body.command.r#type {
        shared_protocol::CommandType::BuildRoad | shared_protocol::CommandType::PlaceRoad => {
            let req: Result<BuildRoadRequest, _> = serde_json::from_value(body.command.payload.clone());
            match req {
                Ok(r) => match guard.roads.apply_build(r) {
                    Ok(edge_id) => {
                        let dto = guard.roads.to_dto();
                        let v = serde_json::to_value(&dto).unwrap();
                        if let Err(e) = save_road_graph(&state.db, city_id, &v).await {
                            error!("save_road_graph failed: {}", e);
                        }
                        CommandResult::ok(body.command.id, Some(serde_json::json!({"edge_id": edge_id, "applied":true})))
                    },
                    Err(e) => CommandResult::err(body.command.id, e),
                },
                Err(e) => CommandResult::err(body.command.id, format!("invalid payload: {e}")),
            }
        },
        shared_protocol::CommandType::ZoneArea => {
            let req: Result<CreateZoneRequest, _> = serde_json::from_value(body.command.payload.clone());
            match req {
                Ok(r) => match create_zone(&state.db, city_id, r).await {
                    Ok(z) => CommandResult::ok(body.command.id, Some(serde_json::to_value(&z).unwrap())),
                    Err(e) => CommandResult::err(body.command.id, e.to_string()),
                },
                Err(e) => CommandResult::err(body.command.id, format!("invalid payload: {e}")),
            }
        },
        _ => CommandResult::ok(body.command.id, Some(serde_json::json!({"ack":true}))),
    };
    guard.world.clock.tick();
    Ok(Json(result))
}

async fn save_city_handler(State(state): State<AppState>, Path(city_id): Path<CityId>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let wrapper = get_or_load_city(&state, city_id).await?;
    let guard = wrapper.lock().await;
    save_city_state(&state.db, city_id, &guard.world).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let dto = guard.roads.to_dto();
    let v = serde_json::to_value(&dto).unwrap();
    save_road_graph(&state.db, city_id, &v).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({"status":"saved","id":city_id,"tick":guard.world.clock.tick})))
}

// Roads REST
async fn get_roads_handler(State(state): State<AppState>, Path(city_id): Path<CityId>) -> Result<Json<RoadGraphDto>, (StatusCode, String)> {
    let wrapper = get_or_load_city(&state, city_id).await?;
    let guard = wrapper.lock().await;
    Ok(Json(guard.roads.to_dto()))
}

async fn build_road_handler(State(state): State<AppState>, Path(city_id): Path<CityId>, Json(req): Json<BuildRoadRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let wrapper = get_or_load_city(&state, city_id).await?;
    let mut guard = wrapper.lock().await;
    match guard.roads.apply_build(req) {
        Ok(edge_id) => {
            let dto = guard.roads.to_dto();
            let v = serde_json::to_value(&dto).unwrap();
            save_road_graph(&state.db, city_id, &v).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(Json(serde_json::json!({"status":"ok","edge_id": edge_id, "road_graph": dto})))
        },
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

// Zones / Parcels (spec 10)
async fn list_zones_handler(State(state): State<AppState>, Path(city_id): Path<CityId>) -> Result<Json<Vec<ZoneDto>>, (StatusCode, String)> {
    let zones = list_zones(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(zones))
}
async fn create_zone_handler(State(state): State<AppState>, Path(city_id): Path<CityId>, Json(req): Json<CreateZoneRequest>) -> Result<Json<ZoneDto>, (StatusCode, String)> {
    // Validate city exists
    get_city(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "city not found".into()))?;
    let zone = create_zone(&state.db, city_id, req).await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(zone))
}
async fn delete_zone_handler(State(state): State<AppState>, Path((city_id, zone_id)): Path<(CityId, i64)>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    delete_zone(&state.db, city_id, zone_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({"status":"deleted","zone_id":zone_id})))
}
async fn list_parcels_handler(State(state): State<AppState>, Path(city_id): Path<CityId>) -> Result<Json<Vec<ParcelDto>>, (StatusCode, String)> {
    let parcels = list_parcels(&state.db, city_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(parcels))
}

// Chunk - procedural generation with persistence fallback (spec 7.2)
#[derive(Deserialize)]
struct ChunkPath { id: CityId, cx: i32, cy: i32 }

async fn get_chunk_handler(State(state): State<AppState>, Path(p): Path<ChunkPath>) -> Result<Json<ChunkDto>, (StatusCode, String)> {
    let city_id = p.id;
    let wrapper = get_or_load_city(&state, city_id).await?;
    let guard = wrapper.lock().await;
    // 1) Check in-memory chunks
    if let Some(ch) = guard.world.chunks.iter().find(|c| c.x == p.cx && c.y == p.cy) {
        return Ok(Json(ChunkDto{ cx: ch.x, cy: ch.y, data: ch.data.clone() }));
    }
    // 2) Check persistence (sparse delta)
    // For MVP we skip DB chunk lookup for procedural chunks and generate directly
    // 3) Procedural generation
    let seed = guard.world.seed;
    drop(guard);
    let chunk = generate_chunk(seed, p.cx, p.cy);
    // Return as DTO without persisting (sparse)
    Ok(Json(ChunkDto{ cx: chunk.x, cy: chunk.y, data: chunk.data }))
}

// ---------------------- WS ----------------------
async fn ws_handler(State(state): State<AppState>, Path(city_id): Path<CityId>, ws: WebSocketUpgrade) -> Result<impl IntoResponse, (StatusCode, String)> {
    let _ = get_or_load_city(&state, city_id).await?;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, city_id)))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, city_id: CityId) {
    let wrapper = match get_or_load_city(&state, city_id).await {
        Ok(w) => w,
        Err((_, msg)) => { let _ = socket.send(Message::Text(serde_json::to_string(&ServerMessage::Error{message:msg}).unwrap())).await; return; }
    };
    let snapshot = {
        let (meta, chunks, road_graph, tick) = {
            let guard = wrapper.lock().await;
            let meta = get_city(&state.db, city_id).await.ok().flatten().unwrap_or(CityMeta{id:city_id,name:guard.city_name.clone(),seed:guard.world.seed,tick:guard.world.clock.tick,created_at:"".into()});
            let chunks: Vec<ChunkDto> = guard.world.chunks.iter().map(|c| ChunkDto{cx:c.x,cy:c.y,data:c.data.clone()}).collect();
            (meta, chunks, guard.roads.to_dto(), guard.world.clock.tick)
        };
        let zones = list_zones(&state.db, city_id).await.unwrap_or_default();
        let parcels = list_parcels(&state.db, city_id).await.unwrap_or_default();
        ServerMessage::Snapshot(InitialSnapshot{city:meta, tick, chunks, road_graph, zones, parcels})
    };
    if socket.send(Message::Text(serde_json::to_string(&snapshot).unwrap())).await.is_err() { return; }

    while let Some(Ok(msg)) = socket.next().await {
        let txt = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        let client_msg: Result<ClientMessage, _> = serde_json::from_str(&txt);
        match client_msg {
            Ok(ClientMessage::Command(cmd)) => {
                // Apply BuildRoad via WS as well
                let applied = match cmd.r#type {
                    shared_protocol::CommandType::BuildRoad | shared_protocol::CommandType::PlaceRoad => {
                        if let Ok(req) = serde_json::from_value::<BuildRoadRequest>(cmd.payload.clone()) {
                            let mut guard = wrapper.lock().await;
                            match guard.roads.apply_build(req) {
                                Ok(edge_id) => {
                                    let dto = guard.roads.to_dto();
                                    let v = serde_json::to_value(&dto).unwrap();
                                    let _ = save_road_graph(&state.db, city_id, &v).await;
                                    let delta = ServerMessage::Delta(shared_protocol::WorldDelta{city_id, tick: guard.world.clock.tick, changed_chunks: vec![], changed_roads: Some(dto), changed_zones: None, changed_parcels: None, events: vec![serde_json::json!({"type":"RoadBuilt","edge_id":edge_id})]});
                                    let _ = socket.send(Message::Text(serde_json::to_string(&delta).unwrap())).await;
                                    CommandResult::ok(cmd.id, Some(serde_json::json!({"edge_id":edge_id})))
                                },
                                Err(e) => CommandResult::err(cmd.id, e),
                            }
                        } else { CommandResult::err(cmd.id, "invalid BuildRoad payload") }
                    },
                    shared_protocol::CommandType::ZoneArea => {
                        if let Ok(req) = serde_json::from_value::<CreateZoneRequest>(cmd.payload.clone()) {
                            match create_zone(&state.db, city_id, req).await {
                                Ok(z) => {
                                    let zones = list_zones(&state.db, city_id).await.unwrap_or_default();
                                    let parcels = list_parcels(&state.db, city_id).await.unwrap_or_default();
                                    let delta = ServerMessage::Delta(shared_protocol::WorldDelta{city_id, tick: 0, changed_chunks: vec![], changed_roads: None, changed_zones: Some(zones), changed_parcels: Some(parcels), events: vec![serde_json::json!({"type":"ZoneCreated","zone":z})]});
                                    let _ = socket.send(Message::Text(serde_json::to_string(&delta).unwrap())).await;
                                    CommandResult::ok(cmd.id, Some(serde_json::to_value(&z).unwrap()))
                                },
                                Err(e) => CommandResult::err(cmd.id, e.to_string()),
                            }
                        } else { CommandResult::err(cmd.id, "invalid ZoneArea payload") }
                    },
                    _ => CommandResult::ok(cmd.id, Some(serde_json::json!({"city_id":city_id}))),
                };
                let _ = socket.send(Message::Text(serde_json::to_string(&ServerMessage::Result(applied)).unwrap())).await;
            },
            Ok(ClientMessage::Ping{t}) => {
                let _ = socket.send(Message::Text(serde_json::to_string(&ServerMessage::Pong{t}).unwrap())).await;
            },
            Ok(ClientMessage::RequestChunk(req)) => {
                let (chunkDto, tick) = {
                    let guard = wrapper.lock().await;
                    if let Some(ch) = guard.world.chunks.iter().find(|c| c.x==req.cx && c.y==req.cy) {
                        (ChunkDto{cx: ch.x, cy: ch.y, data: ch.data.clone()}, guard.world.clock.tick)
                    } else {
                        let seed = guard.world.seed;
                        let tick = guard.world.clock.tick;
                        drop(guard);
                        let ch = generate_chunk(seed, req.cx, req.cy);
                        (ChunkDto{cx: ch.x, cy: ch.y, data: ch.data}, tick)
                    }
                };
                let resp = ServerMessage::Delta(shared_protocol::WorldDelta{city_id, tick, changed_chunks: vec![chunkDto], changed_roads: None, changed_zones: None, changed_parcels: None, events: vec![]});
                let _ = socket.send(Message::Text(serde_json::to_string(&resp).unwrap())).await;
            },
            Ok(ClientMessage::Subscribe{city_id:_, radius:_}) => {
                let _ = socket.send(Message::Text(serde_json::to_string(&ServerMessage::Error{message:"subscribe ack".into()}).unwrap())).await;
            },
            Err(e) => {
                let _ = socket.send(Message::Text(serde_json::to_string(&ServerMessage::Error{message: e.to_string()}).unwrap())).await;
            }
        }
    }
}

// ---------------------- Main ----------------------
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cfg = Config::default();
    let db_path = std::env::var("URBANIA_DB").unwrap_or(cfg.database_path.clone());
    let db_url = if db_path.contains("://") { db_path.clone() } else { format!("sqlite://{}?mode=rwc", db_path) };
    let db = SqlitePool::connect(&db_url).await?;
    init_db(&db).await?;
    let cities_meta = list_cities(&db).await?;
    let mut map: CityMap = HashMap::new();
    for meta in cities_meta {
        let state = load_city_state(&db, meta.id).await?.unwrap_or(SimulationState{ seed: meta.seed, rng: DeterministicRng::from_seed(meta.seed), clock: SimClock{tick: meta.tick, time_ms: meta.tick * TICK_MS}, chunks: vec![] });
        let road_json = load_road_graph(&db, meta.id).await?.unwrap_or(serde_json::json!({"nodes":[],"edges":[]}));
        let dto: RoadGraphDto = serde_json::from_value(road_json).unwrap_or_default();
        let roads = RoadGraph::from_dto(dto);
        let wrapper = WorldWrapper::from_state(meta.id, meta.name.clone(), state, roads);
        map.insert(meta.id, Arc::new(Mutex::new(wrapper)));
    }
    if map.is_empty() {
        let seed = { use rand::RngCore; let mut b=[0u8;8]; rand::rngs::OsRng.fill_bytes(&mut b); u64::from_le_bytes(b) };
        let meta = create_city(&db, "New City", seed).await?;
        let wrapper = WorldWrapper::new(meta.id, meta.name.clone(), meta.seed, 0);
        map.insert(meta.id, Arc::new(Mutex::new(wrapper)));
        info!("Created default city id={}", meta.id);
    }
    let shared_cities = Arc::new(Mutex::new(map));
    let app_state = AppState { db: db.clone(), cities: shared_cities.clone() };

    let tick_cities = shared_cities.clone();
    let tick_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));
        let mut counter: u64 = 0;
        loop {
            interval.tick().await;
            let mut to_persist: Vec<(CityId, u64)> = Vec::new();
            {
                let map = tick_cities.lock().await;
                for (id, w) in map.iter() {
                    let mut guard = w.lock().await;
                    guard.tick(Duration::from_millis(TICK_MS));
                    if guard.world.clock.tick % 100 == 0 {
                        to_persist.push((*id, guard.world.clock.tick));
                    }
                }
            }
            counter += 1;
            if counter % 100 == 0 {
                for (id, tick) in to_persist {
                    if let Err(e) = update_city_tick(&tick_db, id, tick).await {
                        error!("Failed to persist tick for city {}: {}", id, e);
                    }
                }
            }
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/cities", get(list_cities_handler).post(create_city_handler))
        .route("/cities/:id", get(get_city_handler).delete(delete_city_handler))
        .route("/cities/:id/command", post(command_handler))
        .route("/cities/:id/save", post(save_city_handler))
        .route("/cities/:id/roads", get(get_roads_handler).post(build_road_handler))
        .route("/cities/:id/zones", get(list_zones_handler).post(create_zone_handler))
        .route("/cities/:id/zones/:zone_id", axum::routing::delete(delete_zone_handler))
        .route("/cities/:id/parcels", get(list_parcels_handler))
        .route("/cities/:id/chunks/:cx/:cy", get(get_chunk_handler))
        .route("/cities/:id/ws", get(ws_handler))
        .with_state(app_state);

    let addr = format!("0.0.0.0:{}", std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(cfg.port));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Urbania server listening on {} (db={})", addr, db_url);
    axum::serve(listener, app).await?;
    Ok(())
}
