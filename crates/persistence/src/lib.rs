//! Persistence crate - SQLite multi-city schema.
//! Implements incremental snapshots per spec sec 40.

use shared_protocol::{CityId, CityMeta, ParcelDto, ZoneDto, ZoneType, CreateZoneRequest};
use sim_core::{Chunk, DeterministicRng, SimClock, SimulationState, TICK_MS};
use sqlx::SqlitePool;

pub async fn init_db(pool: &SqlitePool) -> sqlx::Result<()> {
    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON;").execute(pool).await?;
    // Cities table - authoritative per city (spec 40.1)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            seed INTEGER NOT NULL,
            tick INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;
    // Chunks - sparse delta storage (spec 7.2)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city_chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            city_id INTEGER NOT NULL,
            cx INTEGER NOT NULL,
            cy INTEGER NOT NULL,
            data BLOB NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(city_id) REFERENCES cities(id) ON DELETE CASCADE,
            UNIQUE(city_id, cx, cy)
        );
        "#,
    )
    .execute(pool)
    .await?;
    // Legacy tables kept for migration compatibility (world_meta/chunks)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS world_meta (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            seed INTEGER NOT NULL,
            tick INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            cx INTEGER NOT NULL,
            cy INTEGER NOT NULL,
            data BLOB NOT NULL,
            meta_id INTEGER NOT NULL,
            FOREIGN KEY(meta_id) REFERENCES world_meta(id)
        );
        "#,
    )
    .execute(pool)
    .await?;
    // Historical metrics (compressed per spec 46) - minimal stub
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS historical_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            city_id INTEGER NOT NULL,
            tick INTEGER NOT NULL,
            metrics TEXT NOT NULL,
            FOREIGN KEY(city_id) REFERENCES cities(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;
    // Road graph per city (spec 9) - JSON blob
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city_road_graph (
            city_id INTEGER PRIMARY KEY,
            data TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(city_id) REFERENCES cities(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;
    // Zones / Parcels / Buildings (spec 10-11)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city_zones (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            city_id INTEGER NOT NULL,
            x1 INTEGER NOT NULL,
            y1 INTEGER NOT NULL,
            x2 INTEGER NOT NULL,
            y2 INTEGER NOT NULL,
            zone_type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(city_id) REFERENCES cities(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city_parcels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            city_id INTEGER NOT NULL,
            zone_id INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            w INTEGER NOT NULL,
            h INTEGER NOT NULL,
            FOREIGN KEY(city_id) REFERENCES cities(id) ON DELETE CASCADE,
            FOREIGN KEY(zone_id) REFERENCES city_zones(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS city_buildings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parcel_id INTEGER NOT NULL,
            archetype TEXT NOT NULL,
            height REAL NOT NULL,
            footprint TEXT NOT NULL,
            FOREIGN KEY(parcel_id) REFERENCES city_parcels(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// City CRUD
// ---------------------------------------------------------------------------

pub async fn create_city(pool: &SqlitePool, name: &str, seed: u64) -> sqlx::Result<CityMeta> {
    let res = sqlx::query(
        "INSERT INTO cities (name, seed, tick, created_at, updated_at) VALUES (?, ?, 0, datetime('now'), datetime('now'))",
    )
    .bind(name)
    .bind(seed as i64)
    .execute(pool)
    .await?;
    let id = res.last_insert_rowid();
    get_city(pool, id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
}

pub async fn get_city(pool: &SqlitePool, id: CityId) -> sqlx::Result<Option<CityMeta>> {
    let row = sqlx::query_as::<_, (i64, String, i64, i64, String)>(
        "SELECT id, name, seed, tick, created_at FROM cities WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, name, seed, tick, created_at)| CityMeta {
        id,
        name,
        seed: seed as u64,
        tick: tick as u64,
        created_at,
    }))
}

pub async fn list_cities(pool: &SqlitePool) -> sqlx::Result<Vec<CityMeta>> {
    let rows = sqlx::query_as::<_, (i64, String, i64, i64, String)>(
        "SELECT id, name, seed, tick, created_at FROM cities ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, seed, tick, created_at)| CityMeta {
            id,
            name,
            seed: seed as u64,
            tick: tick as u64,
            created_at,
        })
        .collect())
}

pub async fn update_city_tick(pool: &SqlitePool, id: CityId, tick: u64) -> sqlx::Result<()> {
    sqlx::query("UPDATE cities SET tick = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(tick as i64)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_city(pool: &SqlitePool, id: CityId) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM cities WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Chunk persistence (sparse delta)
// ---------------------------------------------------------------------------

pub async fn save_city_chunks(pool: &SqlitePool, city_id: CityId, chunks: &[Chunk]) -> sqlx::Result<()> {
    for chunk in chunks {
        let data = serde_json::to_vec(chunk).unwrap();
        sqlx::query(
            r#"
            INSERT INTO city_chunks (city_id, cx, cy, data, updated_at)
            VALUES (?, ?, ?, ?, datetime('now'))
            ON CONFLICT(city_id, cx, cy) DO UPDATE SET data = excluded.data, updated_at = datetime('now')
            "#,
        )
        .bind(city_id)
        .bind(chunk.x)
        .bind(chunk.y)
        .bind(data)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn load_city_chunks(pool: &SqlitePool, city_id: CityId) -> sqlx::Result<Vec<Chunk>> {
    let rows = sqlx::query_as::<_, (i32, i32, Vec<u8>)>(
        "SELECT cx, cy, data FROM city_chunks WHERE city_id = ?",
    )
    .bind(city_id)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for (cx, cy, blob) in rows {
        let mut chunk: Chunk = serde_json::from_slice(&blob).unwrap();
        chunk.x = cx;
        chunk.y = cy;
        out.push(chunk);
    }
    Ok(out)
}

pub async fn load_city_state(pool: &SqlitePool, city_id: CityId) -> sqlx::Result<Option<SimulationState>> {
    let meta = match get_city(pool, city_id).await? {
        Some(m) => m,
        None => return Ok(None),
    };
    let chunks = load_city_chunks(pool, city_id).await?;
    let state = SimulationState {
        seed: meta.seed,
        rng: DeterministicRng::from_seed(meta.seed),
        clock: SimClock { tick: meta.tick, time_ms: meta.tick * TICK_MS },
        chunks,
    };
    Ok(Some(state))
}

pub async fn save_city_state(pool: &SqlitePool, city_id: CityId, state: &SimulationState) -> sqlx::Result<()> {
    update_city_tick(pool, city_id, state.clock.tick).await?;
    save_city_chunks(pool, city_id, &state.chunks).await?;
    Ok(())
}

pub async fn save_road_graph(pool: &SqlitePool, city_id: CityId, graph_json: &serde_json::Value) -> sqlx::Result<()> {
    let data = serde_json::to_string(graph_json).unwrap();
    sqlx::query(
        r#"
        INSERT INTO city_road_graph (city_id, data, updated_at)
        VALUES (?, ?, datetime('now'))
        ON CONFLICT(city_id) DO UPDATE SET data = excluded.data, updated_at = datetime('now')
        "#,
    )
    .bind(city_id)
    .bind(data)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_road_graph(pool: &SqlitePool, city_id: CityId) -> sqlx::Result<Option<serde_json::Value>> {
    let row = sqlx::query_as::<_, (String,)>("SELECT data FROM city_road_graph WHERE city_id = ?",)
        .bind(city_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(s,)| serde_json::from_str(&s).unwrap()))
}

// Zones / Parcels / Buildings

fn zone_type_to_str(t: ZoneType) -> &'static str {
    match t {
        ZoneType::ResidentialLow => "ResidentialLow",
        ZoneType::ResidentialMedium => "ResidentialMedium",
        ZoneType::ResidentialHigh => "ResidentialHigh",
        ZoneType::Commercial => "Commercial",
        ZoneType::Office => "Office",
        ZoneType::Industrial => "Industrial",
        ZoneType::MixedUse => "MixedUse",
        ZoneType::Park => "Park",
    }
}
fn str_to_zone_type(s: &str) -> ZoneType {
    match s {
        "ResidentialLow" => ZoneType::ResidentialLow,
        "ResidentialMedium" => ZoneType::ResidentialMedium,
        "ResidentialHigh" => ZoneType::ResidentialHigh,
        "Commercial" => ZoneType::Commercial,
        "Office" => ZoneType::Office,
        "Industrial" => ZoneType::Industrial,
        "MixedUse" => ZoneType::MixedUse,
        "Park" => ZoneType::Park,
        _ => ZoneType::ResidentialLow,
    }
}

pub async fn create_zone(pool: &SqlitePool, city_id: CityId, req: CreateZoneRequest) -> sqlx::Result<ZoneDto> {
    let (x1, y1, x2, y2) = (req.x1.min(req.x2), req.y1.min(req.y2), req.x1.max(req.x2), req.y1.max(req.y2));
    if x1 == x2 || y1 == y2 { return Err(sqlx::Error::Protocol("zero-area zone".into())); }
    if (x2 - x1).abs() > 1000 || (y2 - y1).abs() > 1000 { return Err(sqlx::Error::Protocol("zone too large".into())); }
    let zt = zone_type_to_str(req.zone_type);
    let res = sqlx::query("INSERT INTO city_zones (city_id, x1, y1, x2, y2, zone_type, created_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))")
        .bind(city_id).bind(x1).bind(y1).bind(x2).bind(y2).bind(zt)
        .execute(pool).await?;
    let zone_id = res.last_insert_rowid();
    // Generate parcels - subdivide into 2x2 cells (spec 10 parcel generation)
    let pw = 2i64;
    let ph = 2i64;
    for y in (y1..y2).step_by(ph as usize) {
        for x in (x1..x2).step_by(pw as usize) {
            let w = (pw).min(x2 - x);
            let h = (ph).min(y2 - y);
            if w > 0 && h > 0 {
                sqlx::query("INSERT INTO city_parcels (city_id, zone_id, x, y, w, h) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(city_id).bind(zone_id).bind(x).bind(y).bind(w).bind(h)
                    .execute(pool).await?;
            }
        }
    }
    // Fetch created zone
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, String, String)>("SELECT id, city_id, x1, y1, x2, y2, zone_type, created_at FROM city_zones WHERE id = ?",)
        .bind(zone_id).fetch_one(pool).await?;
    Ok(ZoneDto{ id: row.0, city_id: row.1, x1: row.2, y1: row.3, x2: row.4, y2: row.5, zone_type: str_to_zone_type(&row.6), created_at: row.7 })
}

pub async fn list_zones(pool: &SqlitePool, city_id: CityId) -> sqlx::Result<Vec<ZoneDto>> {
    let rows = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, String, String)>("SELECT id, city_id, x1, y1, x2, y2, zone_type, created_at FROM city_zones WHERE city_id = ? ORDER BY id",)
        .bind(city_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(id, cid, x1, y1, x2, y2, zt, ca)| ZoneDto{ id, city_id: cid, x1, y1, x2, y2, zone_type: str_to_zone_type(&zt), created_at: ca }).collect())
}

pub async fn list_parcels(pool: &SqlitePool, city_id: CityId) -> sqlx::Result<Vec<ParcelDto>> {
    let rows = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>("SELECT id, zone_id, x, y, w, h FROM city_parcels WHERE city_id = ? ORDER BY id",)
        .bind(city_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(id, zid, x, y, w, h)| ParcelDto{ id, zone_id: zid, x, y, w, h }).collect())
}

pub async fn delete_zone(pool: &SqlitePool, city_id: CityId, zone_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM city_zones WHERE id = ? AND city_id = ?").bind(zone_id).bind(city_id).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn multi_city_round_trip() -> sqlx::Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_db(&pool).await.unwrap();
        let c1 = create_city(&pool, "Alpha", 123).await.unwrap();
        let c2 = create_city(&pool, "Beta", 456).await.unwrap();
        assert_ne!(c1.id, c2.id);
        let list = list_cities(&pool).await.unwrap();
        assert_eq!(list.len(), 2);
        let chunk = Chunk { x: 0, y: 0, data: serde_json::json!({"h":1}) };
        save_city_chunks(&pool, c1.id, &[chunk.clone()]).await.unwrap();
        let loaded = load_city_chunks(&pool, c1.id).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].x, 0);
        let empty = load_city_chunks(&pool, c2.id).await.unwrap();
        assert!(empty.is_empty());
        Ok(())
    }
}
