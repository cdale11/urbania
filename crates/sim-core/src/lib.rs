//! Simulation core library
//!
//! Provides deterministic simulation primitives, a fixed‑step clock, and a simple RNG wrapper.

use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use noise::{NoiseFn, Perlin, Seedable};

/// Fixed simulation tick duration in milliseconds.
pub const TICK_MS: u64 = 100; // 10 Hz default simulation speed

/// Deterministic random number generator wrapper.
#[derive(Debug, Clone)]
pub struct DeterministicRng {
    rng: StdRng,
}

impl DeterministicRng {
    /// Create a new RNG from a 64‑bit seed.
    pub fn from_seed(seed: u64) -> Self {
        // Expand the 64‑bit seed to 256‑bit for StdRng by using the seed as the first element.
        let mut seed_bytes = [0u8; 32];
        seed_bytes[..8].copy_from_slice(&seed.to_le_bytes());
        let rng = StdRng::from_seed(seed_bytes);
        Self { rng }
    }

    /// Generate a random `f64` in the range [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        use rand::Rng;
        self.rng.gen()
    }
}

/// Default RNG seeded with zero for deserialization fallback.
impl Default for DeterministicRng {
    fn default() -> Self {
        Self::from_seed(0)
    }
}

/// Simple deterministic simulation clock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimClock {
    /// Simulation tick counter.
    pub tick: u64,
    /// Current simulation time in milliseconds.
    pub time_ms: u64,
}

impl SimClock {
    /// Initialise a new clock at tick 0.
    pub fn new() -> Self {
        Self { tick: 0, time_ms: 0 }
    }

    /// Advance the clock by one fixed tick.
    pub fn tick(&mut self) {
        self.tick += 1;
        self.time_ms = self.tick * TICK_MS;
    }
}

/// Initialise the simulation core with a seed.
/// Returns a tuple of `(DeterministicRng, SimClock)`.
pub fn init(seed: u64) -> (DeterministicRng, SimClock) {
    (DeterministicRng::from_seed(seed), SimClock::new())
}

// --- Phase 0 additional skeletons ---------------------------------------------------

// Placeholder world chunk representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub x: i32,
    pub y: i32,
    // Arbitrary payload, e.g., terrain data.
    pub data: serde_json::Value,
}

// Overall simulation state (seed, RNG, clock, and world chunks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationState {
    pub seed: u64,
    #[serde(skip)]
    pub rng: DeterministicRng,
    pub clock: SimClock,
    pub chunks: Vec<Chunk>,
}

// Initialise a full simulation state with an empty world.
pub fn init_state(seed: u64) -> SimulationState {
    SimulationState {
        seed,
        rng: DeterministicRng::from_seed(seed),
        clock: SimClock::new(),
        chunks: Vec::new(),
    }
}

// Save the simulation state to a JSON file.
// Returns a Result<(), std::io::Error>.
pub fn save_state(state: &SimulationState, path: &std::path::Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(path, json)
}

// Load a simulation state from a JSON file.
pub fn load_state(path: &std::path::Path) -> std::io::Result<SimulationState> {
    let json = std::fs::read_to_string(path)?;
    let state = serde_json::from_str(&json)?;
    Ok(state)
}

// WebAssembly export: generate a deterministic height map using Perlin noise.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_generate_height_map(seed: u64, offset_x: i32, offset_y: i32, size: usize) -> Box<[f32]> {
    // Create a Perlin noise generator seeded for reproducibility.
    let perlin = Perlin::new(seed as u32);
    // Frequency scaling factor; adjust as needed.
    let scale = 0.1_f64;
    let mut heights = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let nx = (offset_x as f64 + x as f64) * scale;
            let ny = (offset_y as f64 + y as f64) * scale;
            // Sample noise, map from [-1,1] to [0,1].
            let value = (perlin.get([nx, ny]) as f32 * 0.5) + 0.5;
            heights.push(value);
        }
    }
    heights.into_boxed_slice()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_reproducibility() {
        let seed = 42u64;
        let mut rng1 = DeterministicRng::from_seed(seed);
        let mut rng2 = DeterministicRng::from_seed(seed);
        assert_eq!(rng1.next_f64(), rng2.next_f64());
    }

    #[test]
    fn clock_advances_correctly() {
        let mut clock = SimClock::new();
        clock.tick();
        assert_eq!(clock.tick, 1);
        assert_eq!(clock.time_ms, TICK_MS);
        clock.tick();
        assert_eq!(clock.tick, 2);
        assert_eq!(clock.time_ms, 2 * TICK_MS);
    }
}
