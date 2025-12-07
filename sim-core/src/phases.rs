//! High-level simulation phases for the tree–attractor system.
//!
//! The typical update loop looks like:
//! 1. [`attraction_phase`] — each attractor pulls on nearby nodes,
//!    accumulating influence directions in an [`InfluenceBuffer`].
//! 2. [`growth_phase`] — the tree grows new nodes in the averaged
//!    influence directions (plus optional tropism).
//! 3. [`kill_phase`] — attractors that are close enough to nodes are
//!    marked as consumed (killed) and stop participating.

use crate::{
    attractor::AttractorSet, config::Config, influence_buffer::InfluenceBuffer, tree::Tree,
    types::NodeId,
};

/// Accumulates attraction from alive attractors onto nearby tree nodes.
///
/// For each alive attractor:
///
/// 1. Calls [`Tree::find_kth_nearest_nodes`] to find a nearby node and
///    the squared distance to it.
/// 2. If the distance is within `cfg.influence_radius`, normalizes the
///    vector from the node to the attractor and adds it into the
///    [`InfluenceBuffer`] for that node.
/// 3. Sets `Attractor::owner` to the node id if it is influenced, or
///    to `None` otherwise.
///
/// The influence buffer is resized (and cleared) to `tree.nodes.len()`
/// at the start of this phase via [`InfluenceBuffer::ensure_len`].
///
/// ### Parameters
/// - `tree` - The current tree structure; only read access is required.
/// - `attractors` - Set of attractors; their `owner` fields are updated
///   depending on which node attracts them.
/// - `cfg` - Global configuration, providing the influence radius and
///   the `k` index (`Config::attract_from_kn`) used when calling
///   [`Tree::find_kth_nearest_nodes`] to choose an owning node for
///   each attractor.
/// - `acc` - Scratch buffer used to accumulate influence directions per node.
pub fn attraction_phase(
    tree: &Tree,
    attractors: &mut AttractorSet,
    cfg: &Config,
    acc: &mut InfluenceBuffer,
) {
    // Squared influence radius for distance comparison.
    let r2 = cfg.influence_radius * cfg.influence_radius;

    // Make sure the buffer matches the current tree size and is clear.
    acc.ensure_len(tree.nodes.len());

    // Iterate over alive attractors only.
    for a in attractors.points.iter_mut().filter(|a| a.alive) {
        if let Some((id, d2)) = tree.find_kth_nearest_nodes(a.pos, cfg.attract_from_kn) {
            if d2 < r2 {
                // Direction from node to attractor.
                let dir = (a.pos - tree.nodes[id].pos).normalize_or_zero();
                acc.add(id, dir);
                a.owner = Some(id);
            } else {
                a.owner = None;
            }
        } else {
            a.owner = None;
        }
    }
}

/// Grows the tree by adding new child nodes in the influenced directions.
///
/// For each node that has at least one influence in the
/// [`InfluenceBuffer`]:
///
/// 1. Compute the average influence direction using
///    [`InfluenceBuffer::avg_dir`].
/// 2. Normalize it, add the global [`Config::tropism`] bias, and
///    normalize again.
/// 3. Propose a new node at:
///    `new_pos = old_pos + dir * cfg.step_len`.
/// 4. Skip if there is already a child near `new_pos` according to
///    [`Tree::has_child_near`].
/// 5. Otherwise, add a child node via [`Tree::add_child`] and remember
///    its id.
///
/// The function returns all newly created node ids in the order they
/// were added.
///
/// ### Parameters
/// - `tree` - The tree to be mutated; new nodes will be appended.
/// - `acc` - The accumulated influence buffer from [`attraction_phase`].
/// - `cfg` - Global configuration defining step length and tropism.
///
/// ### Returns
/// A vector of [`NodeId`] values corresponding to newly created nodes.
pub fn growth_phase(tree: &mut Tree, acc: &InfluenceBuffer, cfg: &Config) -> Vec<NodeId> {
    let mut new_ids = Vec::with_capacity(16);
    let mut to_add = Vec::with_capacity(16);

    // For each influenced node, compute a growth direction and a candidate child.
    for id in acc.influenced_indices() {
        let mut dir = acc.avg_dir(id);

        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }

        // Apply global tropism (e.g. gravity / wind) and renormalize.
        dir += cfg.tropism;
        dir = dir.normalize_or_zero();

        // Proposed new node position.
        let new_pos = tree.nodes[id].pos + dir * cfg.step_len;
        let new_radius = tree.nodes[id].radius;

        // Avoid spawning children that are too close to existing ones.
        if tree.has_child_near(id, new_pos, 0.1) {
            continue;
        }

        to_add.push((id, new_pos, new_radius));
    }

    // Actually add nodes to the tree and collect their ids.
    for (p, pos, r) in to_add {
        new_ids.push(tree.add_child(p, pos, r));
    }
    new_ids
}

/// Marks attractors as consumed (killed) if they are close to the tree.
///
/// For each alive attractor:
///
/// 1. Uses [`Tree::find_kth_nearest_nodes`] with `cfg.kill_from_kn` to
///    find a nearby node and distance squared.
/// 2. If that distance is within `cfg.kill_radius`, the attractor is
///    marked as dead by setting `alive = false`.
///
/// This phase usually runs **after** [`growth_phase`], so that attractors
/// near newly created nodes are removed and stop influencing later steps.
///
/// ### Parameters
/// - `tree` - The current tree; only read access is required.
/// - `attractors` - Attractor set; some attractors will be marked as dead.
/// - `cfg` - Global configuration, providing the kill radius and the
///   `k` index (`Config::kill_from_kn`) used when looking up the
///   k-th nearest node to each attractor.
pub fn kill_phase(tree: &Tree, attractors: &mut AttractorSet, cfg: &Config) {
    let r2 = cfg.kill_radius * cfg.kill_radius;
    for a in attractors.points.iter_mut().filter(|a| a.alive) {
        if let Some((_id, d2)) = tree.find_kth_nearest_nodes(a.pos, cfg.kill_from_kn)
            && d2 < r2
        {
            a.alive = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        attractor::AttractorSet, config::Config, influence_buffer::InfluenceBuffer, tree::Tree,
    };
    use glam::Vec2;

    #[test]
    fn attraction_phase_accumulates_influence_and_sets_owner() {
        // A simple tree with a single root at (0, 0).
        let tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);

        // One attractor at (1, 0), definitely within the influence radius.
        let mut attractors = AttractorSet::from_positions(vec![Vec2::new(1.0, 0.0)]);

        let mut cfg = Config::default();
        cfg.influence_radius = 2.0;
        cfg.attract_from_kn = 0;

        let mut acc = InfluenceBuffer::with_len(0);

        attraction_phase(&tree, &mut attractors, &cfg, &mut acc);

        // Buffer should be resized to match the number of tree nodes.
        assert_eq!(acc.count.len(), tree.nodes.len());

        // Root node should receive exactly one contribution.
        assert_eq!(acc.count[0], 1);

        // Direction should be from root (0,0) to attractor (1,0), i.e. (1,0).
        let dir = acc.avg_dir(0);
        assert_eq!(dir, Vec2::new(1.0, 0.0));

        // Attractor owner should be the root node (id 0).
        assert_eq!(attractors.points[0].owner, Some(0));
    }

    #[test]
    fn attraction_phase_does_not_influence_outside_radius() {
        let tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);

        // A far-away attractor.
        let mut attractors = AttractorSet::from_positions(vec![Vec2::new(100.0, 0.0)]);

        let mut cfg = Config::default();
        cfg.influence_radius = 1.0; // too small to reach the attractor
        cfg.attract_from_kn = 0;

        let mut acc = InfluenceBuffer::with_len(0);

        attraction_phase(&tree, &mut attractors, &cfg, &mut acc);

        // Still resized to 1 node.
        assert_eq!(acc.count.len(), 1);
        // But no influence should be recorded.
        assert_eq!(acc.count[0], 0);
        // Owner should be None.
        assert_eq!(attractors.points[0].owner, None);
    }

    #[test]
    fn growth_phase_creates_child_in_influence_direction() {
        let mut tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        let mut acc = InfluenceBuffer::with_len(1);

        // Add a rightward influence (1, 0) to the root node.
        acc.add(0, Vec2::new(1.0, 0.0));

        let mut cfg = Config::default();
        cfg.step_len = 2.0;
        cfg.tropism = Vec2::new(0.0, 0.0);

        let new_ids = growth_phase(&mut tree, &acc, &cfg);

        // Exactly one new node should be created.
        assert_eq!(new_ids.len(), 1);
        let child_id = new_ids[0];
        assert_eq!(child_id, 1);
        assert_eq!(tree.nodes.len(), 2);

        let child = &tree.nodes[child_id];
        // New node should be at (2, 0) given step_len = 2.0 and direction (1, 0).
        assert_eq!(child.pos, Vec2::new(2.0, 0.0));
        // Radius should be inherited from the parent.
        assert_eq!(child.radius, tree.nodes[0].radius);
        // Parent's children list should contain this child.
        assert_eq!(tree.nodes[0].children, vec![child_id]);
    }

    #[test]
    fn growth_phase_skips_when_child_already_near() {
        let mut tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        let mut acc = InfluenceBuffer::with_len(1);

        // Root node has a rightward influence.
        acc.add(0, Vec2::new(1.0, 0.0));

        let mut cfg = Config::default();
        cfg.step_len = 2.0;
        cfg.tropism = Vec2::new(0.0, 0.0);

        // Manually add a child at the expected new position (2, 0).
        let existing_child_id = tree.add_child(0, Vec2::new(2.0, 0.0), 1.0);
        assert_eq!(existing_child_id, 1);

        let new_ids = growth_phase(&mut tree, &acc, &cfg);

        // Because there is already a child near the candidate position,
        // no additional child should be created.
        assert!(new_ids.is_empty());
        assert_eq!(tree.nodes.len(), 2);
    }

    #[test]
    fn kill_phase_marks_attractors_inside_radius_as_dead() {
        let tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);

        // One close attractor and one far attractor.
        let mut attractors = AttractorSet::from_positions(vec![
            Vec2::new(0.0, 1.0),  // distance 1
            Vec2::new(10.0, 0.0), // far away
        ]);

        let mut cfg = Config::default();
        cfg.kill_radius = 2.0; // radius large enough to cover the first attractor
        cfg.kill_from_kn = 0;

        kill_phase(&tree, &mut attractors, &cfg);

        assert!(
            !attractors.points[0].alive,
            "first attractor should be killed"
        );
        assert!(
            attractors.points[1].alive,
            "second attractor should remain alive"
        );
    }

    #[test]
    fn kill_phase_with_empty_tree_does_not_panic_or_kill() {
        // Manually construct an empty tree.
        let tree = Tree { nodes: Vec::new() };

        let mut attractors =
            AttractorSet::from_positions(vec![Vec2::new(0.0, 1.0), Vec2::new(2.0, 0.0)]);

        let mut cfg = Config::default();
        cfg.kill_radius = 2.0;
        cfg.kill_from_kn = 0;

        kill_phase(&tree, &mut attractors, &cfg);

        // With no nodes, find_kth_nearest_nodes returns None, so
        // no attractors should be killed.
        assert!(attractors.points.iter().all(|a| a.alive));
    }
}
