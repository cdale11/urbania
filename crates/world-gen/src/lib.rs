//! World-gen crate - deterministic terrain chunks (spec 7-8)
//! Generates sparse procedural base; persistence stores only deltas.

use noise::{NoiseFn, Perlin, Seedable};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sim_core::Chunk;

pub const CHUNK_SIZE: usize = 16;
pub const TERRAIN_SCALE: f64 = 0.1;

/// Generate heights for a chunk at (cx,cy) using world seed deterministically.
/// Mirrors `sim-core::wasm_generate_height_map` perlin logic for consistency.
pub fn generate_heights(seed: u64, cx: i32, cy: i32, size: usize) -> Vec<f32> {
    let perlin = Perlin::new(seed as u32);
    let offset_x = cx * size as i32;
    let offset_y = cy * size as i32;
    let mut out = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let nx = (offset_x + x as i32) as f64 * TERRAIN_SCALE;
            let ny = (offset_y + y as i32) as f64 * TERRAIN_SCALE;
            let v = (perlin.get([nx, ny]) as f32 * 0.5) + 0.5;
            out.push(v);
        }
    }
    out
}

/// Generate a full Chunk with JSON payload {heights, water, vegetation, seed, cx, cy}
pub fn generate_chunk(seed: u64, cx: i32, cy: i32) -> Chunk {
    let heights = generate_heights(seed, cx, cy, CHUNK_SIZE);
    // Water mask: height <0.30
    let water: Vec<bool> = heights.iter().map(|h| *h < 0.30).collect();
    // Vegetation suitability: 0.35-0.70 + second perlin octave
    let veg_perlin = Perlin::new((seed.wrapping_add(1000)) as u32);
    let offset_x = cx * CHUNK_SIZE as i32;
    let offset_y = cy * CHUNK_SIZE as i32;
    let mut vegetation = Vec::with_capacity(CHUNK_SIZE*CHUNK_SIZE);
    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let idx = y*CHUNK_SIZE + x;
            let h = heights[idx];
            let nx = (offset_x + x as i32) as f64 * TERRAIN_SCALE * 0.5;
            let ny = (offset_y + y as i32) as f64 * TERRAIN_SCALE * 0.5;
            let veg_noise = (veg_perlin.get([nx, ny]) as f32 * 0.5) + 0.5;
            let veg = if h >= 0.35 && h <= 0.70 && veg_noise > 0.55 { veg_noise } else { 0.0 };
            vegetation.push(veg);
        }
    }
    let data = json!({
        "heights": heights,
        "water": water,
        "vegetation": vegetation,
        "size": CHUNK_SIZE,
        "seed": seed,
        "cx": cx,
        "cy": cy,
        "kind": "terrain"
    });
    Chunk { x: cx, y: cy, data }
}

/// Deterministic check - same seed+coords must give identical chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkHeights {
    pub heights: Vec<f32>,
    pub size: usize,
}

pub fn chunk_heights_from_chunk(chunk: &Chunk) -> Option<Vec<f32>> {
    chunk.data.get("heights")?.as_array().map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic() {
        let a = generate_chunk(42, 0, 0);
        let b = generate_chunk(42, 0, 0);
        assert_eq!(a.data, b.data);
        let c = generate_chunk(42, 1, 0);
        assert_ne!(a.data, c.data);
    }
    #[test]
    fn range_ok() {
        let h = generate_heights(123, -2, 5, 16);
        assert_eq!(h.len(), 256);
        for v in h { assert!((0.0..=1.0).contains(&v)); }
    }
    #[test]
    fn adjacent_continuity() {
        // Heights at edge should be continuous if we generate a 32-wide map vs two 16 chunks
        let big = generate_heights(7, 0, 0, 32);
        let left = generate_heights(7, 0, 0, 16);
        // Compare first 16 rows of big with left (first half)
        for y in 0..16 {
            for x in 0..16 {
                let bi = y*32 + x;
                let li = y*16 + x;
                assert!((big[bi] - left[li]).abs() < 1e-6);
            }
        }
    }
}
