//! Transport crate - deterministic RoadGraph (spec sec 9)
//! Uses shared-protocol DTOs as canonical wire format.

use serde::{Deserialize, Serialize};
use shared_protocol::{BuildRoadRequest, JunctionType, RoadEdgeDto, RoadGraphDto, RoadNodeDto, WorldPos};
use sim_core::DeterministicRng;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoadGraph {
    pub graph: RoadGraphDto,
    #[serde(skip)]
    next_id: u64,
}

impl RoadGraph {
    pub fn new() -> Self { Self { graph: RoadGraphDto::default(), next_id: 1 } }

    pub fn from_dto(dto: RoadGraphDto) -> Self {
        let max_id = dto.nodes.iter().map(|n| n.id).chain(dto.edges.iter().map(|e| e.id)).max().unwrap_or(0);
        Self { graph: dto, next_id: max_id + 1 }
    }

    pub fn to_dto(&self) -> RoadGraphDto { self.graph.clone() }

    fn alloc_id(&mut self) -> u64 { let id = self.next_id; self.next_id += 1; id }

    const SNAP_DIST: f64 = 5.0;

    fn find_or_create_node(&mut self, pos: WorldPos) -> u64 {
        for n in &self.graph.nodes {
            let dx = (n.pos.x - pos.x) as f64;
            let dy = (n.pos.y - pos.y) as f64;
            if (dx*dx + dy*dy).sqrt() < Self::SNAP_DIST { return n.id; }
        }
        let id = self.alloc_id();
        self.graph.nodes.push(RoadNodeDto{ id, pos, junction: JunctionType::End });
        id
    }

    pub fn validate_build(&self, cmd: &BuildRoadRequest) -> Result<(), String> {
        if cmd.from == cmd.to { return Err("zero-length road".into()); }
        let dx = (cmd.to.x - cmd.from.x) as f64;
        let dy = (cmd.to.y - cmd.from.y) as f64;
        let len = (dx*dx + dy*dy).sqrt();
        if len > 10000.0 { return Err("road too long".into()); }
        if let Some(lanes) = cmd.lanes { if lanes==0 || lanes>8 { return Err("lanes 1..8".into()); } }
        Ok(())
    }

    pub fn apply_build(&mut self, cmd: BuildRoadRequest) -> Result<u64, String> {
        self.validate_build(&cmd)?;
        let start = self.find_or_create_node(cmd.from);
        let end = self.find_or_create_node(cmd.to);
        for e in &self.graph.edges { if (e.start==start && e.end==end) || (e.start==end && e.end==start) { return Err("edge already exists".into()); } }
        let id = self.alloc_id();
        let lanes = cmd.lanes.unwrap_or(2);
        let speed = match lanes { 1 => 30, 2 => 50, _ => 70 };
        self.graph.edges.push(RoadEdgeDto{ id, start, end, lanes, speed_limit: speed, width: lanes as f32 * 3.5, grade: 0.0 });
        self.recompute_junctions();
        Ok(id)
    }

    fn recompute_junctions(&mut self) {
        let mut degree: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
        for e in &self.graph.edges { *degree.entry(e.start).or_default() += 1; *degree.entry(e.end).or_default() += 1; }
        for n in &mut self.graph.nodes {
            let d = *degree.get(&n.id).unwrap_or(&0);
            n.junction = match d { 0|1 => JunctionType::End, 2 => JunctionType::Straight, _ => JunctionType::Intersection };
        }
    }

    pub fn generate_grid(&mut self, seed: u64, origin: WorldPos, w: usize, h: usize, spacing: i64) {
        let mut rng = DeterministicRng::from_seed(seed);
        for y in 0..h {
            for x in 0..w {
                let jitter = ((rng.next_f64()-0.5)*2.0) as i64;
                let px = origin.x + x as i64 * spacing + jitter;
                let py = origin.y + y as i64 * spacing + jitter;
                let pos = WorldPos{ x: px, y: py };
                if x+1 < w {
                    let to = WorldPos{ x: px + spacing, y: py };
                    let _ = self.apply_build(BuildRoadRequest{ from: pos, to, lanes: Some(2) });
                }
                if y+1 < h {
                    let to = WorldPos{ x: px, y: py + spacing };
                    let _ = self.apply_build(BuildRoadRequest{ from: pos, to, lanes: Some(2) });
                }
            }
        }
    }

    pub fn invariants_ok(&self) -> Result<(), String> {
        let ids: std::collections::HashSet<u64> = self.graph.nodes.iter().map(|n| n.id).collect();
        for e in &self.graph.edges {
            if e.start == e.end { return Err(format!("self-loop edge {}", e.id)); }
            if !ids.contains(&e.start) || !ids.contains(&e.end) { return Err(format!("edge {} references missing node", e.id)); }
        }
        Ok(())
    }
}

// Keep legacy type aliases for any external code expecting old names
pub use shared_protocol::{WorldPos as TransportPos, JunctionType as TransportJunction};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_and_validate() {
        let mut g = RoadGraph::new();
        let id = g.apply_build(BuildRoadRequest{ from: WorldPos{x:0,y:0}, to: WorldPos{x:100,y:0}, lanes: Some(2)}).unwrap();
        assert!(g.invariants_ok().is_ok());
        assert!(g.apply_build(BuildRoadRequest{ from: WorldPos{x:0,y:0}, to: WorldPos{x:100,y:0}, lanes: Some(2)}).is_err());
        assert!(g.validate_build(&BuildRoadRequest{ from: WorldPos{x:0,y:0}, to: WorldPos{x:0,y:0}, lanes: None}).is_err());
        assert_eq!(g.graph.edges[0].id, id);
    }
    #[test]
    fn deterministic_grid() {
        let mut a = RoadGraph::new(); a.generate_grid(42, WorldPos{x:0,y:0}, 3, 3, 100);
        let mut b = RoadGraph::new(); b.generate_grid(42, WorldPos{x:0,y:0}, 3, 3, 100);
        assert_eq!(a.graph.nodes.len(), b.graph.nodes.len());
        assert_eq!(a.graph.edges.len(), b.graph.edges.len());
    }
    #[test]
    fn dto_round_trip() {
        let mut g = RoadGraph::new();
        g.apply_build(BuildRoadRequest{ from: WorldPos{x:0,y:0}, to: WorldPos{x:10,y:0}, lanes: None}).unwrap();
        let dto = g.to_dto();
        let g2 = RoadGraph::from_dto(dto.clone());
        assert_eq!(g2.graph.nodes.len(), dto.nodes.len());
    }
}
