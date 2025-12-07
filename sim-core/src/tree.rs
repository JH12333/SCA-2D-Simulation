use crate::types::NodeId;
use glam::Vec2;

/// A single node in the tree structure.
///
/// Each node stores its position, radius (thickness), an optional parent
/// reference, and a list of children. The tree as a whole is stored in a
/// contiguous `Vec<TreeNode>`, and [`NodeId`] is used as the index.
///
/// ### Fields
/// - `pos` - World-space position of this node.
/// - `radius` - Radius or thickness of the branch at this node.
/// - `parent` - Optional parent node ID; `None` for root / free nodes.
/// - `children` - IDs of this node's direct children.
#[derive(Debug)]
pub struct TreeNode {
    pub pos: Vec2,
    pub radius: f32,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

/// A simple tree of nodes stored in a flat array.
///
/// Nodes are indexed by [`NodeId`] (typically an index into `nodes`), and
/// parent–child relations are tracked via `parent` and `children` fields
/// in each [`TreeNode`].
///
/// The root node is usually created via [`Tree::new`], but additional
/// “free” roots can be added using [`Tree::add_free_node`].
#[derive(Debug)]
pub struct Tree {
    pub nodes: Vec<TreeNode>,
}

impl TreeNode {
    /// Creates a new root node with no parent.
    ///
    /// The node is initialized with an empty `children` list.
    ///
    /// ### Parameters
    /// - `pos` - Position of the root node.
    /// - `radius` - Branch radius / thickness at this node.
    ///
    /// ### Returns
    /// A [`TreeNode`] with `parent = None`.
    pub fn new_root(pos: Vec2, radius: f32) -> Self {
        Self {
            pos,
            radius,
            parent: None,
            children: Vec::with_capacity(4),
        }
    }

    /// Creates a new child node with a given parent.
    ///
    /// The node itself does not update the parent's `children` list;
    /// this is handled by [`Tree::add_child`].
    ///
    /// ### Parameters
    /// - `pos` - Position of the child node.
    /// - `radius` - Branch radius / thickness at this node.
    /// - `parent` - Parent node ID.
    ///
    /// ### Returns
    /// A [`TreeNode`] whose `parent` is set to `Some(parent)`.
    pub fn new_child(pos: Vec2, radius: f32, parent: NodeId) -> Self {
        Self {
            pos,
            radius,
            parent: Some(parent),
            children: Vec::with_capacity(4),
        }
    }
}

impl Tree {
    /// Creates a new tree with a single root node.
    ///
    /// The root node is placed at `root_pos` with the given `root_radius`
    /// and stored at index `0` in the `nodes` array.
    ///
    /// ### Parameters
    /// - `root_pos` - Position of the root node.
    /// - `root_radius` - Branch radius / thickness of the root node.
    ///
    /// ### Returns
    /// A [`Tree`] containing exactly one node at index `0`.
    pub fn new(root_pos: Vec2, root_radius: f32) -> Self {
        Self {
            nodes: vec![TreeNode::new_root(root_pos, root_radius)],
        }
    }

    /// Adds a new “free” node that has no parent.
    ///
    /// This is effectively another root in the forest stored inside `Tree`.
    ///
    /// ### Parameters
    /// - `pos` - Position of the new node.
    /// - `radius` - Branch radius / thickness at this node.
    ///
    /// ### Returns
    /// The [`NodeId`] (index) of the newly added node.
    pub fn add_free_node(&mut self, pos: Vec2, radius: f32) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(TreeNode::new_root(pos, radius));
        id
    }

    /// Adds a new child node under the given parent.
    ///
    /// This method:
    /// - Appends a new [`TreeNode`] to `nodes` with `parent = Some(parent)`.
    /// - Pushes the new node's id into `parent`'s `children` list.
    ///
    /// ### Parameters
    /// - `parent` - ID of the parent node.
    /// - `pos` - Position of the new child node.
    /// - `radius` - Branch radius / thickness at this node.
    ///
    /// ### Returns
    /// The [`NodeId`] (index) of the newly added child node.
    pub fn add_child(&mut self, parent: NodeId, pos: Vec2, radius: f32) -> NodeId {
        let id: usize = self.nodes.len();
        self.nodes.push(TreeNode::new_child(pos, radius, parent));
        self.nodes[parent].children.push(id);
        id
    }

    /// Checks whether the given parent already has a child near `pos`.
    ///
    /// The check is performed using squared distance:
    ///
    /// - Let `eps2 = eps * eps`.
    /// - For each child `c` of `parent`, compute
    ///   `d2 = length_squared(nodes[c].pos - pos)`.
    /// - Returns `true` if any `d2 < eps2`, `false` otherwise.
    ///
    /// ### Parameters
    /// - `parent` - ID of the parent node whose children will be checked.
    /// - `pos` - Candidate position to test.
    /// - `eps` - Distance threshold within which a child is considered “near”.
    ///
    /// ### Returns
    /// `true` if there is at least one nearby child, `false` otherwise.
    pub fn has_child_near(&self, parent: NodeId, pos: Vec2, eps: f32) -> bool {
        let eps2 = eps * eps;
        self.nodes[parent].children.iter().any(|&cid| {
            let d2 = (self.nodes[cid].pos - pos).length_squared();
            d2 < eps2
        })
    }

    /// Finds the node nearest to the given position.
    ///
    /// The search is a simple linear scan over all nodes in the tree,
    /// and returns the index and squared distance of the closest node.
    ///
    /// If the tree has no nodes, `None` is returned.
    ///
    /// ### Parameters
    /// - `pos` - Query position.
    ///
    /// ### Returns
    /// - `Some((id, dist2))` where `id` is the nearest node and `dist2` is
    ///   the squared distance to `pos`, or
    /// - `None` if there are no nodes.
    pub fn find_nearest_node(&self, pos: Vec2) -> Option<(NodeId, f32)> {
        let mut best = None;
        let mut best_d2 = f32::MAX;

        for (id, n) in self.nodes.iter().enumerate() {
            let d2 = (n.pos - pos).length_squared();
            if d2 < best_d2 {
                best_d2 = d2;
                best = Some(id);
            }
        }

        best.map(|id| (id, best_d2))
    }

    /// Finds the *k*-th nearest node to the given position.
    ///
    /// This function builds a list of `(id, dist2)` pairs for all nodes,
    /// then uses `select_nth_unstable_by` to partially sort by distance.
    ///
    /// ### Semantics
    /// - Nodes are ordered by increasing squared distance to `pos`.
    /// - If `k < n` (where `n = nodes.len()`), returns the node at index `k`
    ///   in that ordered list (0 = nearest, 1 = second nearest, etc.).
    /// - If `k >= n`, returns the *farthest* node (i.e. the `(n - 1)`-th).
    /// - If there are no nodes (`n == 0`), returns `None`.
    ///
    /// ### Parameters
    /// - `pos` - Query position.
    /// - `k` - Zero-based rank of the nearest node to retrieve.
    ///
    /// ### Returns
    /// - `Some((id, dist2))` with the selected node id and squared distance, or
    /// - `None` if there are no nodes.
    pub fn find_kth_nearest_nodes(&self, pos: Vec2, k: usize) -> Option<(NodeId, f32)> {
        let n = self.nodes.len();
        if n == 0 {
            return None;
        }

        let mut dist_list: Vec<(NodeId, f32)> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(id, node)| {
                let d = node.pos - pos;
                let dist2 = d.length_squared();
                (id, dist2)
            })
            .collect();

        if k >= n {
            dist_list.select_nth_unstable_by(n - 1, |a, b| a.1.total_cmp(&b.1));
            return Some(dist_list[n - 1]);
        }
        dist_list.select_nth_unstable_by(k, |a, b| a.1.total_cmp(&b.1));
        Some(dist_list[k])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn new_tree_creates_single_root() {
        let root_pos = Vec2::new(0.0, 1.0);
        let root_radius = 2.0;
        let tree = Tree::new(root_pos, root_radius);

        assert_eq!(tree.nodes.len(), 1);
        let root = &tree.nodes[0];
        assert_eq!(root.pos, root_pos);
        assert_eq!(root.radius, root_radius);
        assert!(root.parent.is_none());
        assert!(root.children.is_empty());
    }

    #[test]
    fn add_child_links_parent_and_child() {
        let mut tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        let parent_id: NodeId = 0;
        let child_pos = Vec2::new(1.0, 0.0);
        let child_radius = 0.5;

        let child_id = tree.add_child(parent_id, child_pos, child_radius);

        assert_eq!(child_id, 1);
        assert_eq!(tree.nodes.len(), 2);

        let parent = &tree.nodes[parent_id];
        assert_eq!(parent.children, vec![child_id]);

        let child = &tree.nodes[child_id];
        assert_eq!(child.pos, child_pos);
        assert_eq!(child.radius, child_radius);
        assert_eq!(child.parent, Some(parent_id));
        assert!(child.children.is_empty());
    }

    #[test]
    fn add_free_node_creates_orphan_node() {
        let mut tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        let pos = Vec2::new(5.0, 5.0);
        let radius = 0.8;

        let id = tree.add_free_node(pos, radius);

        assert_eq!(id, 1);
        assert_eq!(tree.nodes.len(), 2);

        let node = &tree.nodes[id];
        assert_eq!(node.pos, pos);
        assert_eq!(node.radius, radius);
        assert!(node.parent.is_none());
        assert!(node.children.is_empty());
    }

    #[test]
    fn has_child_near_detects_close_child() {
        let mut tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        let parent_id: NodeId = 0;
        let child_pos = Vec2::new(1.0, 0.0);
        tree.add_child(parent_id, child_pos, 0.5);

        // Close to existing child.
        assert!(tree.has_child_near(parent_id, Vec2::new(1.05, 0.0), 0.2));
        // Far away from existing child.
        assert!(!tree.has_child_near(parent_id, Vec2::new(2.0, 0.0), 0.2));
    }

    #[test]
    fn find_nearest_node_returns_none_for_empty_tree() {
        let tree = Tree { nodes: Vec::new() };
        let pos = Vec2::new(0.0, 0.0);

        let result = tree.find_nearest_node(pos);
        assert!(result.is_none());
    }

    #[test]
    fn find_nearest_node_finds_closest_node() {
        let mut tree = Tree::new(Vec2::new(0.0, 0.0), 1.0); // id 0
        tree.add_free_node(Vec2::new(10.0, 0.0), 1.0); // id 1
        tree.add_free_node(Vec2::new(2.0, 0.0), 1.0); // id 2

        let pos = Vec2::new(1.5, 0.0);
        let (id, d2) = tree.find_nearest_node(pos).unwrap();

        assert_eq!(id, 2);
        let expected_d2 = (tree.nodes[id].pos - pos).length_squared();
        assert!((d2 - expected_d2).abs() < 1e-6);
    }

    #[test]
    fn find_kth_nearest_nodes_basic_behavior() {
        let mut tree = Tree::new(Vec2::new(0.0, 0.0), 1.0); // id 0
        tree.add_free_node(Vec2::new(1.0, 0.0), 1.0); // id 1
        tree.add_free_node(Vec2::new(3.0, 0.0), 1.0); // id 2

        let pos = Vec2::new(0.5, 0.0);

        // Distances:
        // id 0: |0.0 - 0.5|^2 = 0.25
        // id 1: |1.0 - 0.5|^2 = 0.25
        // id 2: |3.0 - 0.5|^2 = 6.25

        // k = 0 → one of the nearest (0 or 1)
        let (id0, _) = tree.find_kth_nearest_nodes(pos, 0).unwrap();
        assert!(id0 == 0 || id0 == 1);

        // k = 1 → the other nearest (0 or 1)
        let (id1, _) = tree.find_kth_nearest_nodes(pos, 1).unwrap();
        assert!(id1 == 0 || id1 == 1);
        assert_ne!(id0, id1);

        // k >= n → farthest (id 2)
        let (idf, _) = tree.find_kth_nearest_nodes(pos, 5).unwrap();
        assert_eq!(idf, 2);
    }

    #[test]
    fn find_kth_nearest_nodes_empty_returns_none() {
        let tree = Tree { nodes: Vec::new() };
        let pos = Vec2::new(0.0, 0.0);

        assert!(tree.find_kth_nearest_nodes(pos, 0).is_none());
        assert!(tree.find_kth_nearest_nodes(pos, 10).is_none());
    }
}
