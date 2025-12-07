//! Core 2-D tree growth and attractor simulation library.
//!
//! Main components:
//! - [`attractor`] — attractor points and sets.
//! - [`tree`] — tree nodes and growth logic.
//! - [`config`] — global configuration for the growth algorithm.
//! - [`influence_buffer`] — temporary buffers for accumulated influences.
//! - [`phases`] — high-level simulation phases / pipeline.
//! - [`types`] — shared type aliases and IDs.

pub mod attractor;
pub mod config;
pub mod influence_buffer;
pub mod phases;
pub mod tree;
pub mod types;
