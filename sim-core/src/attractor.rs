use crate::types::NodeId;
use glam::Vec2;
use rand::Rng;
use std::f32::consts::TAU;

/// A single attractor point used to guide growth or influence in the system.
///
/// Each attractor has a position in 2D space, a liveness flag, and an optional
/// owner (e.g. the node that has claimed or consumed it).
///
/// ### Fields
/// - `pos` - The position of the attractor in world coordinates.
/// - `alive` - Whether this attractor is still active and can be used.
/// - `owner` - Optional ID of the node that owns or has claimed this attractor.
#[derive(Debug)]
pub struct Attractor {
    pub pos: Vec2,
    pub alive: bool,
    pub owner: Option<NodeId>,
}

/// A collection of attractor points.
///
/// This type provides helpers to build attractor sets from explicit positions,
/// or to generate them randomly inside simple shapes (rectangle / oval).
///
/// Typical usage is to generate an `AttractorSet` at the beginning of a
/// simulation step, then let tree nodes or agents query and claim them.
#[derive(Debug)]
pub struct AttractorSet {
    pub points: Vec<Attractor>,
}

impl AttractorSet {
    /// Creates an [`AttractorSet`] from explicit positions.
    ///
    /// Every position becomes an `Attractor` with `alive = true` and
    /// `owner = None`.
    ///
    /// ### Parameters
    /// - `positions` - A list of positions where attractors should be placed.
    ///
    /// ### Returns
    /// A new [`AttractorSet`] containing one attractor per position.
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

    /// Generates attractors uniformly inside an axis-aligned rectangle.
    ///
    /// The rectangle is defined by its `center` and `half_extents`, i.e. the
    /// generated positions `p` satisfy:
    ///
    /// - `center.x - half_extents.x <= p.x <= center.x + half_extents.x`
    /// - `center.y - half_extents.y <= p.y <= center.y + half_extents.y`
    ///
    /// All generated attractors start with `alive = true` and `owner = None`.
    ///
    /// ### Parameters
    /// - `center` - Center of the rectangle region.
    /// - `half_extents` - Half-width and half-height of the rectangle.
    /// - `count` - Number of attractors to generate.
    /// - `rng` - Random number generator used to sample positions.
    ///
    /// ### Returns
    /// An [`AttractorSet`] with `count` randomly placed attractors.
    pub fn random_in_rect(
        center: Vec2,
        half_extents: Vec2,
        count: usize,
        rng: &mut impl Rng,
    ) -> Self {
        let positions = (0..count)
            .map(|_| {
                let x = rng.random_range(-half_extents.x..=half_extents.x);
                let y = rng.random_range(-half_extents.y..=half_extents.y);
                center + Vec2::new(x, y)
            })
            .collect();

        Self::from_positions(positions)
    }

    /// Generates attractors uniformly inside an axis-aligned oval (ellipse).
    ///
    /// The oval is centered at `center` with radii `radii.x` and `radii.y`
    /// along the x and y axes respectively. Points are sampled so that they
    /// are uniformly distributed over the area of the ellipse.
    ///
    /// Implementation details:
    /// - A random angle `θ` is taken from `[0, 2π)`.
    /// - A radius `r` is drawn from `[0, 1)` and square-rooted to get uniform
    ///   area distribution.
    /// - The final offset is `(r * cos θ * radii.x, r * sin θ * radii.y)`.
    ///
    /// All generated attractors start with `alive = true` and `owner = None`.
    ///
    /// ### Parameters
    /// - `center` - Center of the oval region.
    /// - `radii` - Radii of the oval in x and y directions.
    /// - `count` - Number of attractors to generate.
    /// - `rng` - Random number generator used to sample positions.
    ///
    /// ### Returns
    /// An [`AttractorSet`] with `count` randomly placed attractors inside the oval.
    pub fn random_in_oval(center: Vec2, radii: Vec2, count: usize, rng: &mut impl Rng) -> Self {
        let positions = (0..count)
            .map(|_| {
                let angle = rng.random_range(0.0..TAU);
                let r = rng.random_range(0.0f32..1.0f32).sqrt();
                let dx = angle.cos() * r * radii.x;
                let dy = angle.sin() * r * radii.y;
                center + Vec2::new(dx, dy)
            })
            .collect();

        Self::from_positions(positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn from_positions_initializes_attractors_correctly() {
        let positions = vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 2.0)];
        let set = AttractorSet::from_positions(positions.clone());

        assert_eq!(set.points.len(), positions.len());

        for (i, attractor) in set.points.iter().enumerate() {
            assert_eq!(attractor.pos, positions[i]);
            assert!(attractor.alive);
            assert!(attractor.owner.is_none());
        }
    }

    #[test]
    fn random_in_rect_generates_points_within_bounds() {
        let center = Vec2::new(0.0, 0.0);
        let half_extents = Vec2::new(10.0, 5.0);
        let count = 128;
        let mut rng = StdRng::seed_from_u64(42);

        let set = AttractorSet::random_in_rect(center, half_extents, count, &mut rng);

        assert_eq!(set.points.len(), count);

        for attractor in &set.points {
            let p = attractor.pos;
            assert!(
                p.x >= center.x - half_extents.x && p.x <= center.x + half_extents.x,
                "x={} not in [{}, {}]",
                p.x,
                center.x - half_extents.x,
                center.x + half_extents.x
            );
            assert!(
                p.y >= center.y - half_extents.y && p.y <= center.y + half_extents.y,
                "y={} not in [{}, {}]",
                p.y,
                center.y - half_extents.y,
                center.y + half_extents.y
            );
            assert!(attractor.alive);
            assert!(attractor.owner.is_none());
        }
    }

    #[test]
    fn random_in_oval_generates_points_inside_ellipse() {
        let center = Vec2::new(1.0, -2.0);
        let radii = Vec2::new(4.0, 2.0);
        let count = 128;
        let mut rng = StdRng::seed_from_u64(123);

        let set = AttractorSet::random_in_oval(center, radii, count, &mut rng);

        assert_eq!(set.points.len(), count);

        let epsilon = 1e-5;

        for attractor in &set.points {
            let local = attractor.pos - center;
            let normalized = (local.x / radii.x).powi(2) + (local.y / radii.y).powi(2);

            // Should be inside or very slightly outside due to float rounding.
            assert!(
                normalized <= 1.0 + epsilon,
                "point {:?} outside ellipse (normalized={})",
                attractor.pos,
                normalized
            );
            assert!(attractor.alive);
            assert!(attractor.owner.is_none());
        }
    }
}
