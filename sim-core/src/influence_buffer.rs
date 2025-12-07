use crate::types::NodeId;
use glam::Vec2;

/// A temporary buffer that accumulates directional influence per node.
///
/// For each `NodeId`, this buffer stores:
///
/// - The sum of all incoming direction vectors.
/// - The number of contributions that were added.
///
/// This allows you to efficiently accumulate influences from attractors
/// (or other nodes) and later query the **average** direction for each node.
///
/// Internally, `dir[i]` and `count[i]` correspond to node `i` (where
/// `NodeId` is expected to be an index-like type, e.g. `usize`).
#[derive(Debug)]
pub struct InfluenceBuffer {
    /// Accumulated direction vectors for each node.
    dir: Vec<Vec2>,
    /// Number of contributions for each node.
    pub count: Vec<u32>,
}

impl InfluenceBuffer {
    /// Creates a new [`InfluenceBuffer`] with the given length.
    ///
    /// All accumulated directions are initialized to `Vec2::ZERO`,
    /// and all counts are initialized to `0`.
    ///
    /// ### Parameters
    /// - `len` - Number of nodes this buffer can store influences for.
    ///
    /// ### Returns
    /// A new [`InfluenceBuffer`] of length `len`.
    pub fn with_len(len: usize) -> Self {
        Self {
            dir: vec![Vec2::ZERO; len],
            count: vec![0; len],
        }
    }

    /// Ensures that the internal storage has exactly the given length.
    ///
    /// If the current length differs from `len`, both the direction and
    /// count arrays are resized to `len`. After this call, all entries
    /// are cleared (directions set to `Vec2::ZERO`, counts set to `0`),
    /// even if the length was already correct.
    ///
    /// ### Parameters
    /// - `len` - Desired length of the internal buffers.
    pub fn ensure_len(&mut self, len: usize) {
        if self.dir.len() != len {
            self.dir.resize(len, Vec2::ZERO);
            self.count.resize(len, 0);
        }
        self.clear();
    }

    /// Clears all accumulated influences.
    ///
    /// After calling this method, all directions are set to `Vec2::ZERO`,
    /// and all counts are reset to `0`, but the length remains unchanged.
    pub fn clear(&mut self) {
        for v in &mut self.dir {
            *v = Vec2::ZERO;
        }
        for c in &mut self.count {
            *c = 0;
        }
    }

    /// Adds one directional influence for the given node.
    ///
    /// The `dir` vector is added to the accumulated direction for this `id`,
    /// and the count for that node is incremented by one.
    ///
    /// ### Parameters
    /// - `id` - Node ID to accumulate influence for (used as an index).
    /// - `dir` - Direction vector to add.
    ///
    /// ### Panics
    /// Panics if `id` is out of bounds for the internal arrays.
    #[inline]
    pub fn add(&mut self, id: NodeId, dir: Vec2) {
        self.dir[id] += dir;
        self.count[id] += 1;
    }

    /// Returns the average influence direction for a node.
    ///
    /// If the node has received no influences (i.e. its count is `0`),
    /// this method returns `Vec2::ZERO`.
    ///
    /// ### Parameters
    /// - `id` - Node ID whose average direction should be queried.
    ///
    /// ### Returns
    /// The average direction vector for the given node, or `Vec2::ZERO`
    /// if no influences were accumulated.
    #[inline]
    pub fn avg_dir(&self, id: NodeId) -> Vec2 {
        let c = self.count[id];
        if c == 0 {
            Vec2::ZERO
        } else {
            self.dir[id] / (c as f32)
        }
    }

    /// Returns `true` if the given node has received any influences.
    ///
    /// This is equivalent to checking whether the count for the node
    /// is greater than zero.
    ///
    /// ### Parameters
    /// - `id` - Node ID to query.
    ///
    /// ### Returns
    /// `true` if `id` has at least one contribution, `false` otherwise.
    #[inline]
    pub fn is_influenced(&self, id: NodeId) -> bool {
        self.count[id] > 0
    }

    /// Returns an iterator over all node indices that have been influenced.
    ///
    /// Only nodes whose count is greater than zero are yielded.
    ///
    /// ### Returns
    /// An iterator of `NodeId` values for which `is_influenced` is `true`.
    pub fn influenced_indices<'a>(&'a self) -> impl Iterator<Item = NodeId> + 'a {
        self.count
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if c > 0 { Some(i) } else { None })
    }

    /// Merges another [`InfluenceBuffer`] into this one.
    ///
    /// For each node `i`, this adds `other.dir[i]` to `self.dir[i]` and
    /// `other.count[i]` to `self.count[i]`.
    ///
    /// ### Parameters
    /// - `other` - The source buffer whose influences will be accumulated
    ///   into `self`.
    ///
    /// ### Panics
    /// Panics if the two buffers have different lengths.
    pub fn merge_from(&mut self, other: &InfluenceBuffer) {
        assert_eq!(self.dir.len(), other.dir.len());
        for i in 0..self.dir.len() {
            self.dir[i] += other.dir[i];
            self.count[i] += other.count[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeId;
    use glam::Vec2;

    #[test]
    fn with_len_initializes_zeroed_state() {
        let len = 5;
        let buf = InfluenceBuffer::with_len(len);

        assert_eq!(buf.dir.len(), len);
        assert_eq!(buf.count.len(), len);

        for v in &buf.dir {
            assert_eq!(*v, Vec2::ZERO);
        }
        for c in &buf.count {
            assert_eq!(*c, 0);
        }
    }

    #[test]
    fn ensure_len_keeps_length_and_clears_when_same() {
        let mut buf = InfluenceBuffer::with_len(3);
        let id: NodeId = 1;
        buf.add(id, Vec2::new(1.0, 2.0));

        assert!(buf.is_influenced(id));

        buf.ensure_len(3);

        assert_eq!(buf.dir.len(), 3);
        assert_eq!(buf.count.len(), 3);
        assert!(!buf.is_influenced(id));
        for v in &buf.dir {
            assert_eq!(*v, Vec2::ZERO);
        }
        for c in &buf.count {
            assert_eq!(*c, 0);
        }
    }

    #[test]
    fn ensure_len_resizes_and_clears_when_different() {
        let mut buf = InfluenceBuffer::with_len(2);
        buf.add(0, Vec2::new(1.0, 0.0));

        buf.ensure_len(4);
        assert_eq!(buf.dir.len(), 4);
        assert_eq!(buf.count.len(), 4);

        for v in &buf.dir {
            assert_eq!(*v, Vec2::ZERO);
        }
        for c in &buf.count {
            assert_eq!(*c, 0);
        }

        buf.ensure_len(1);
        assert_eq!(buf.dir.len(), 1);
        assert_eq!(buf.count.len(), 1);
        assert_eq!(buf.dir[0], Vec2::ZERO);
        assert_eq!(buf.count[0], 0);
    }

    #[test]
    fn clear_resets_all_entries() {
        let mut buf = InfluenceBuffer::with_len(3);
        buf.add(0, Vec2::new(1.0, 0.0));
        buf.add(1, Vec2::new(0.0, 1.0));

        buf.clear();

        for v in &buf.dir {
            assert_eq!(*v, Vec2::ZERO);
        }
        for c in &buf.count {
            assert_eq!(*c, 0);
        }
    }

    #[test]
    fn add_and_avg_dir_work_as_expected() {
        let mut buf = InfluenceBuffer::with_len(2);
        let id: NodeId = 1;

        assert_eq!(buf.avg_dir(id), Vec2::ZERO);
        assert!(!buf.is_influenced(id));

        buf.add(id, Vec2::new(1.0, 0.0));
        buf.add(id, Vec2::new(3.0, 0.0));

        assert!(buf.is_influenced(id));
        assert_eq!(buf.count[id], 2);
        assert_eq!(buf.avg_dir(id), Vec2::new(2.0, 0.0));
    }

    #[test]
    fn influenced_indices_returns_only_nodes_with_nonzero_count() {
        let mut buf = InfluenceBuffer::with_len(4);
        buf.add(0, Vec2::new(1.0, 0.0));
        buf.add(2, Vec2::new(0.0, 1.0));

        let ids: Vec<NodeId> = buf.influenced_indices().collect();
        assert_eq!(ids, vec![0, 2]);

        // After clearing, there should be no influenced nodes.
        buf.clear();
        let ids_after_clear: Vec<NodeId> = buf.influenced_indices().collect();
        assert!(ids_after_clear.is_empty());
    }

    #[test]
    fn merge_from_adds_contributions_of_both_buffers() {
        let mut a = InfluenceBuffer::with_len(3);
        let mut b = InfluenceBuffer::with_len(3);

        a.add(0, Vec2::new(1.0, 0.0));
        a.add(1, Vec2::new(0.0, 1.0));

        b.add(0, Vec2::new(2.0, 0.0));
        b.add(2, Vec2::new(0.0, 3.0));

        a.merge_from(&b);

        assert_eq!(a.dir[0], Vec2::new(3.0, 0.0));
        assert_eq!(a.count[0], 2);

        assert_eq!(a.dir[1], Vec2::new(0.0, 1.0));
        assert_eq!(a.count[1], 1);

        assert_eq!(a.dir[2], Vec2::new(0.0, 3.0));
        assert_eq!(a.count[2], 1);
    }

    #[test]
    #[should_panic]
    fn merge_from_panics_on_mismatched_lengths() {
        let mut a = InfluenceBuffer::with_len(2);
        let b = InfluenceBuffer::with_len(3);
        a.merge_from(&b);
    }
}
