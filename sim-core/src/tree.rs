use crate::types::NodeId;
use glam::Vec2;

#[derive(Debug)]
pub struct TreeNode {
    pub pos: Vec2,
    pub radius: f32,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Debug)]
pub struct Tree {
    pub nodes: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new_root(pos: Vec2, radius: f32) -> Self {
        Self {
            pos,
            radius,
            parent: None,
            children: Vec::with_capacity(4),
        }
    }

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
    pub fn new(root_pos: Vec2, root_radius: f32) -> Self {
        Self {
            nodes: vec![TreeNode::new_root(root_pos, root_radius)],
        }
    }

    pub fn add_child(&mut self, parent: NodeId, pos: Vec2, radius: f32) -> NodeId {
        let id: usize = self.nodes.len();
        self.nodes.push(TreeNode::new_child(pos, radius, parent));
        self.nodes[parent].children.push(id);
        id
    }

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

    pub fn find_kth_nearest_nodes(&self, pos: Vec2, k: usize) -> Option<(NodeId, f32)> {
        let n = self.nodes.len();
        if n == 0 || k >= n {
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

        dist_list.select_nth_unstable_by(k, |a, b| a.1.total_cmp(&b.1));

        Some(dist_list[k])
    }
}
