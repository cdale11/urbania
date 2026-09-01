//! Simulation core library
//!
//! Provides deterministic simulation primitives, a fixed‑step clock, and a simple RNG wrapper.

use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Fixed simulation tick duration in milliseconds.
pub const TICK_MS: u64 = 100; // 10 Hz default simulation speed

/// Deterministic random number generator wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
