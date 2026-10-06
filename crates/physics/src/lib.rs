#![forbid(unsafe_code)]
//! Diagnostic geometry and source-qualified collision family queries.
//! Complete player-world and receiver/solver assembly remains separate.
pub mod brush;
pub mod brush_collection;
pub mod moving_brush_trace;
pub mod moving_leaf_trace;
pub mod static_point_trace;
pub mod stationary_brush_trace;
pub mod stationary_leaf_trace;
mod trace_shape;
pub use bevy::math::{DVec3, Vec3};
