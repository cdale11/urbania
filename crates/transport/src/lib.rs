//! Transport crate - deterministic RoadGraph (spec sec 9)
//! Started as scaffold for Roads & Traffic first system.

use serde::{Deserialize, Serialize};
use sim_core::DeterministicRng;

/// World position in integer simulation coordinates (64-bit per spec 7.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct WorldPos {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JunctionType {
    End,
    Straight,
    Intersection,
    Roundabout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadNode {
    pub id: u64,
    pub pos: WorldPos,
    pub junction: JunctionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadEdge {
    pub id: u64,
    pub start: u64, // node id
    pub end: u64,
    pub lanes: u8,
    pub speed_limit: u16, // km/h
    pub width: f32,
    pub grade: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoadGraph {
    pub nodes: Vec<RoadNode>,
    pub edges: Vec<RoadEdge>,
    #[serde(skip)]
    next_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRoadCommand {
    pub from: WorldPos,
    pub to: WorldPos,
    pub lanes: Option<u8>,
}

impl RoadGraph {
    pub fn new() -> Self { Self { nodes: vec![], edges: vec![], next_id: 1 } }

    fn alloc_id(&mut self) -> u64 { let id = self.next_id; self.next_id += 1; id }

    /// Snap threshold: merge to existing node if within distance.
    const SNAP_DIST: f64 = 5.0;

    fn find_or_create_node(&mut self, pos: WorldPos) -> u64 {
        for n in &self.nodes {
            let dx = (n.pos.x - pos.x) as f64;
            let dy = (n.pos.y - pos.y) as f64;
            if (dx*dx + dy*dy).sqrt() < Self::SNAP_DIST { return n.id; }
        }
        let id = self.alloc_id();
        self.nodes.push(RoadNode{ id, pos, junction: JunctionType::End });
        id
    }

    /// Validate command without mutating (spec 54)
    pub fn validate_build(&self, cmd: &BuildRoadCommand) -> Result<(), String> {
        if cmd.from == cmd.to { return Err("zero-length road".into()); }
        let dx = (cmd.to.x - cmd.from.x) as f64;
        let dy = (cmd.to.y - cmd.from.y) as f64;
        let len = (dx*dx + dy*dy).sqrt();
        if len > 10000.0 { return Err("road too long".into()); }
        if let Some(lanes) = cmd.lanes { if lanes==0 || lanes>8 { return Err("lanes 1..8".into()); } }
        Ok(())
    }

    /// Apply - deterministic, uses no global RNG. Returns created edge id.
    pub fn apply_build(&mut self, cmd: BuildRoadCommand) -> Result<u64, String> {
        self.validate_build(&cmd)?;
        let start = self.find_or_create_node(cmd.from);
        let end = self.find_or_create_node(cmd.to);
        // Prevent duplicate edge
        for e in &self.edges { if (e.start==start && e.end==end) || (e.start==end && e.end==start) { return Err("edge already exists".into()); } }
        let id = self.alloc_id();
        let lanes = cmd.lanes.unwrap_or(2);
        let speed = match lanes { 1 => 30, 2 => 50, _ => 70 };
        self.edges.push(RoadEdge{ id, start, end, lanes, speed_limit: speed, width: lanes as f32 * 3.5, grade: 0.0 });
        self.recompute_junctions();
        Ok(id)
    }

    fn recompute_junctions(&mut self) {
        let mut degree: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
        for e in &self.edges { *degree.entry(e.start).or_default() += 1; *degree.entry(e.end).or_default() += 1; }
        for n in &mut self.nodes {
            let d = *degree.get(&n.id).unwrap_or(&0);
            n.junction = match d { 0|1 => JunctionType::End, 2 => JunctionType::Straight, _ => JunctionType::Intersection };
        }
    }

    /// Deterministic procedural generation helper: generate a grid using seeded RNG (for tests/benchmarks)
    pub fn generate_grid(&mut self, seed: u64, origin: WorldPos, w: usize, h: usize, spacing: i64) {
        let mut rng = DeterministicRng::from_seed(seed);
        for y in 0..h {
            for x in 0..w {
                let jitter = ((rng.next_f64()-0.5)*2.0) as i64;
                let px = origin.x + x as i64 * spacing + jitter;
                let py = origin.y + y as i64 * spacing + jitter;
                let pos = WorldPos{ x: px, y: py };
                // Horizontal edge
                if x+1 < w {
                    let to = WorldPos{ x: px + spacing, y: py };
                    let _ = self.apply_build(BuildRoadCommand{ from: pos, to, lanes: Some(2) });
                }
                if y+1 < h {
                    let to = WorldPos{ x: px, y: py + spacing };
                    let _ = self.apply_build(BuildRoadCommand{ from: pos, to, lanes: Some(2) });
                }
            }
        }
    }

    pub fn invariants_ok(&self) -> Result<(), String> {
        // Every edge references existing nodes, no self-loops
        let ids: std::collections::HashSet<u64> = self.nodes.iter().map(|n| n.id).collect();
        for e in &self.edges {
            if e.start == e.end { return Err(format!("self-loop edge {}", e.id)); }
            if !ids.contains(&e.start) || !ids.contains(&e.end) { return Err(format!("edge {} references missing node", e.id)); }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_and_validate() {
        let mut g = RoadGraph::new();
        let id = g.apply_build(BuildRoadCommand{ from: WorldPos{x:0,y:0}, to: WorldPos{x:100,y:0}, lanes: Some(2)}).unwrap();
        assert!(g.invariants_ok().is_ok());
        // duplicate should fail
        assert!(g.apply_build(BuildRoadCommand{ from: WorldPos{x:0,y:0}, to: WorldPos{x:100,y:0}, lanes: Some(2)}).is_err());
        // zero length
        assert!(g.validate_build(&BuildRoadCommand{ from: WorldPos{x:0,y:0}, to: WorldPos{x:0,y:0}, lanes: None}).is_err());
        assert_eq!(g.edges[0].id, id);
    }
    #[test]
    fn deterministic_grid() {
        let mut a = RoadGraph::new(); a.generate_grid(42, WorldPos{x:0,y:0}, 3, 3, 100);
        let mut b = RoadGraph::new(); b.generate_grid(42, WorldPos{x:0,y:0}, 3, 3, 100);
        assert_eq!(a.nodes.len(), b.nodes.len());
        assert_eq!(a.edges.len(), b.edges.len());
    }
}
