#![forbid(unsafe_code)]
//! Diagnostic brush geometry and native-evidenced single moving-brush queries.
pub mod brush;
pub mod brush_collection;
pub mod moving_brush_trace;
pub use bevy::math::{DVec3, Vec3};
