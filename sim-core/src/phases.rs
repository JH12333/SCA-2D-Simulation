use crate::{
    attractor::AttractorSet, config::Config, influence_buffer::InfluenceBuffer, tree::Tree,
    types::NodeId,
};

pub fn attraction_phase(
    tree: &Tree,
    attractors: &mut AttractorSet,
    cfg: &Config,
    acc: &mut InfluenceBuffer,
) {
    let r2 = cfg.influence_radius * cfg.influence_radius;
    acc.ensure_len(tree.nodes.len());
    for a in attractors.points.iter_mut().filter(|a| a.alive) {
        if let Some((id, d2)) = tree.find_kth_nearest_nodes(a.pos, 0) {
            if d2 < r2 {
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

pub fn growth_phase(tree: &mut Tree, acc: &InfluenceBuffer, cfg: &Config) -> Vec<NodeId> {
    let mut new_ids = Vec::with_capacity(16);
    let mut to_add = Vec::with_capacity(16);

    for id in acc.influenced_indices() {
        let mut dir = acc.avg_dir(id);
        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }
        dir += cfg.tropism;
        dir = dir.normalize_or_zero();
        let new_pos = tree.nodes[id].pos + dir * cfg.step_len;
        let new_radius = tree.nodes[id].radius;
        to_add.push((id, new_pos, new_radius));
    }

    for (p, pos, r) in to_add {
        new_ids.push(tree.add_child(p, pos, r));
    }
    new_ids
}

pub fn kill_phase(tree: &Tree, attractors: &mut AttractorSet, cfg: &Config) {
    let r2 = cfg.kill_radius * cfg.kill_radius;
    for a in attractors.points.iter_mut().filter(|a| a.alive) {
        if let Some((_id, d2)) = tree.find_kth_nearest_nodes(a.pos, 0) {
            if d2 < r2 {
                a.alive = false;
            }
        }
    }
}
