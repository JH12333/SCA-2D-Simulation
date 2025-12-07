use glam::Vec2;

/// Which tool is currently selected for spawning objects in the scene.
///
/// This enum usually backs a UI toggle (e.g. radio buttons or a dropdown)
/// that decides what will be placed when the user clicks in the viewer.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SpawnTool {
    /// Spawn a single root node (e.g. the initial tree/root of the structure).
    RootNode,
    /// Spawn attractors inside a rectangle defined by a center and half-extents.
    RectAttractors,
    /// Spawn attractors inside an oval (ellipse) defined by a center and radii.
    OvalAttractors,
}

/// Global configuration for the tree / attractor system.
///
/// This struct groups all configurable parameters that control
/// how the growth algorithm behaves and how attractors are spawned
/// in the scene.
///
/// It is intended to be cheap to copy so it can be passed around
/// between the core logic and the UI.
///
/// ### Fields
/// - `attract_from_kn` - For each attractor, which nearest tree node
///   to use when assigning its owner. This value is passed as `k` to
///   [`Tree::find_kth_nearest_nodes`]:
///   - `0` → use the closest node,
///   - `1` → use the second-closest node, etc.
///   - if `k >= node_count`, the farthest node is used.
/// - `kill_from_kn` - Same `k` index as above, but used during the
///   kill phase when checking whether an attractor is close enough
///   (within [`Config::kill_radius`]) to be marked as consumed.
/// - `influence_radius` - Maximum distance at which an attractor can
///   influence a node.
/// - `kill_radius` - Distance threshold under which an attractor
///   is considered “consumed” and can be removed.
/// - `step_len` - Step length for each growth update of a node/branch.
/// - `tropism` - Directional bias (e.g. gravity or wind) added to the
///   growth direction.
///
/// - `spawn_tool` - Which spawning mode is currently active in the UI.
/// - `spawn_attractors` - How many attractors to spawn in the chosen shape.
/// - `spawn_rect_half_extents` - Half-extents of the rectangle used when
///   `spawn_tool` is [`SpawnTool::RectAttractors`].
/// - `spawn_oval_radii` - Radii of the oval used when
///   `spawn_tool` is [`SpawnTool::OvalAttractors`].
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub attract_from_kn: usize,
    pub kill_from_kn: usize,
    pub influence_radius: f32,
    pub kill_radius: f32,
    pub step_len: f32,
    pub tropism: Vec2,

    pub spawn_tool: SpawnTool,
    pub spawn_attractors: usize,
    pub spawn_rect_half_extents: Vec2,
    pub spawn_oval_radii: Vec2,
}

impl Default for Config {
    /// Creates a [`Config`] with reasonable defaults for interactive use.
    ///
    /// The default values are tuned for a medium-sized scene where
    /// attractors are placed within a roughly 60×60 area and the
    /// growth step length is small enough to produce smooth branches.
    ///
    /// ### Returns
    /// A [`Config`] instance populated with default parameters.
    fn default() -> Self {
        Self {
            attract_from_kn: 0,
            kill_from_kn: 0,
            influence_radius: 60.0,
            kill_radius: 30.0,
            step_len: 5.0,
            tropism: Vec2::new(0.0, 0.0),

            spawn_tool: SpawnTool::OvalAttractors,
            spawn_attractors: 100,
            spawn_rect_half_extents: Vec2::new(30.0, 30.0),
            spawn_oval_radii: Vec2::new(30.0, 30.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = Config::default();

        // k-NN settings
        assert_eq!(cfg.attract_from_kn, 0);
        assert_eq!(cfg.kill_from_kn, 0);

        // Radii and step length
        assert_eq!(cfg.influence_radius, 60.0);
        assert_eq!(cfg.kill_radius, 30.0);
        assert_eq!(cfg.step_len, 5.0);

        // Tropism
        assert_eq!(cfg.tropism, Vec2::new(0.0, 0.0));

        // Spawn settings
        assert_eq!(cfg.spawn_tool, SpawnTool::OvalAttractors);
        assert_eq!(cfg.spawn_attractors, 100);
        assert_eq!(cfg.spawn_rect_half_extents, Vec2::new(30.0, 30.0));
        assert_eq!(cfg.spawn_oval_radii, Vec2::new(30.0, 30.0));
    }

    #[test]
    fn spawn_tool_equality_and_copy_semantics() {
        let t1 = SpawnTool::RootNode;
        let t2 = t1; // Copy
        assert_eq!(t1, t2);

        let t3 = SpawnTool::RectAttractors;
        assert_ne!(t1, t3);
    }

    #[test]
    fn config_is_copy_and_clone() {
        let cfg = Config::default();
        let mut cfg2 = cfg; // Copy
        cfg2.influence_radius = 80.0;

        // Original should remain unchanged.
        assert_eq!(cfg.influence_radius, 60.0);
        assert_eq!(cfg2.influence_radius, 80.0);

        let cfg3 = cfg2.clone();
        assert_eq!(cfg2.influence_radius, cfg3.influence_radius);
    }

    #[test]
    fn influence_radius_is_not_smaller_than_kill_radius_in_default() {
        let cfg = Config::default();
        assert!(cfg.influence_radius >= cfg.kill_radius);
    }
}
