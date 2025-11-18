use crate::types::NodeId;
use glam::Vec2;

#[derive(Debug)]
pub struct InfluenceBuffer {
    dir: Vec<Vec2>,
    count: Vec<u32>,
}

impl InfluenceBuffer {
    pub fn with_len(len: usize) -> Self {
        Self {
            dir: vec![Vec2::ZERO; len],
            count: vec![0; len],
        }
    }

    pub fn ensure_len(&mut self, len: usize) {
        if self.dir.len() != len {
            self.dir.resize(len, Vec2::ZERO);
            self.count.resize(len, 0);
        } else {
            self.clear();
        }
    }

    pub fn clear(&mut self) {
        for v in &mut self.dir {
            *v = Vec2::ZERO;
        }
        for c in &mut self.count {
            *c = 0;
        }
    }

    #[inline]
    pub fn add(&mut self, id: NodeId, dir: Vec2) {
        self.dir[id] += dir;
        self.count[id] += 1;
    }

    #[inline]
    pub fn avg_dir(&self, id: NodeId) -> Vec2 {
        let c = self.count[id];
        if c == 0 {
            Vec2::ZERO
        } else {
            self.dir[id] / (c as f32)
        }
    }

    #[inline]
    pub fn is_influenced(&self, id: NodeId) -> bool {
        self.count[id] > 0
    }

    pub fn influenced_indices<'a>(&'a self) -> impl Iterator<Item = NodeId> + 'a {
        self.count
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if c > 0 { Some(i) } else { None })
    }

    pub fn merge_from(&mut self, other: &InfluenceBuffer) {
        assert_eq!(self.dir.len(), other.dir.len());
        for i in 0..self.dir.len() {
            self.dir[i] += other.dir[i];
            self.count[i] += other.count[i];
        }
    }
}
