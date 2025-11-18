use crate::types::NodeId;
use glam::Vec2;

#[derive(Debug)]
pub struct Attractor {
    pub pos: Vec2,
    pub alive: bool,
    pub owner: Option<NodeId>,
}

#[derive(Debug)]
pub struct AttractorSet {
    pub points: Vec<Attractor>,
}

impl AttractorSet {
    pub fn from_positions(positions: Vec<Vec2>) -> Self {
        let points = positions
            .into_iter()
            .map(|pos| Attractor {
                pos,
                alive: true,
                owner: None,
            })
            .collect();

        Self { points }
    }
}
