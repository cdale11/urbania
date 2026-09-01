// Placeholder chunk system – in a real implementation this would manage streaming
// of procedurally generated world tiles. Here it simply defines the data shape.

export interface Chunk {
  // Chunk coordinates in the world grid (e.g., tile indices).
  x: number;
  y: number;
  // Arbitrary payload – terrain height map, objects, etc.
  data: Record<string, unknown>;
}

export class ChunkManager {
  private chunks: Map<string, Chunk> = new Map();

  // Simple key generator.
  private key(x: number, y: number) {
    return `${x},${y}`;
  }

  // Retrieve or create a chunk at the given coordinates.
  getOrCreate(x: number, y: number): Chunk {
    const k = this.key(x, y);
    let chunk = this.chunks.get(k);
    if (!chunk) {
      chunk = { x, y, data: {} };
      this.chunks.set(k, chunk);
    }
    return chunk;
  }
}
