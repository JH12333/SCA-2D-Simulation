/// Identifier for a node in a [`crate::tree::Tree`].
///
/// This is an index into `Tree::nodes`, and is only meaningful within
/// the lifetime of a given `Tree` instance.
pub type NodeId = usize;
