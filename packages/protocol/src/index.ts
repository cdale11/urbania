// Shared command protocol between client (TypeScript) and server/simulation (Rust).
// This file defines the minimal set of messages needed for Phase 0.

export type CommandType =
  | 'Init'
  | 'Tick'
  | 'SetPolicy'
  | 'PlaceRoad'
  | 'ZoneArea';

export interface PlayerCommand {
  /** Monotonically increasing command identifier */
  id: number;
  /** Type of command */
  type: CommandType;
  /** Command payload – free‑form JSON */
  payload: Record<string, unknown>;
}

export interface CommandResult {
  /** Echo of the command id */
  id: number;
  /** Success flag */
  ok: boolean;
  /** Optional data returned by the simulation */
  data?: Record<string, unknown>;
}

// Simple serialize/deserialize helpers (JSON based).
export const serializeCommand = (cmd: PlayerCommand): string => JSON.stringify(cmd);
export const deserializeCommand = (s: string): PlayerCommand => JSON.parse(s) as PlayerCommand;
