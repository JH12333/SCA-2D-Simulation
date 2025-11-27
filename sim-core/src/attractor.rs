use crate::types::NodeId;
use glam::Vec2;
use rand::Rng;

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

    pub fn random_in_square(count: usize, half_range: f32, rng: &mut impl Rng) -> Self {
        let positions = (0..count)
            .map(|_| {
                let x = rng.random_range(-half_range..=half_range);
                let y = rng.random_range(-half_range..=half_range);
                Vec2::new(x, y)
            })
            .collect();

        Self::from_positions(positions)
    }
}
