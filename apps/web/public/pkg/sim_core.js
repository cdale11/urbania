export default async function init() {
  // No-op placeholder – real WASM init would go here
}

// Simple deterministic pseudo‑random number generator (mirrors the Rust seed handling)
function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a += 0x6d2b79f5;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function wasm_generate_height_map(seed, x, y, size) {
  // Generate a simple deterministic height map using the mulberry32 PRNG.
  const count = size * size;
  const heights = new Float32Array(count);
  const rng = mulberry32(seed);
  for (let i = 0; i < count; i++) {
    // Scale to a modest range (0‑0.5) for visible terrain variation.
    heights[i] = rng() * 0.5;
  }
  return heights;
}
