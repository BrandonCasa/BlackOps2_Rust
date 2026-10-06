#![forbid(unsafe_code)]

//! Read-only external format boundaries. This crate does not load engine assets.

pub mod appearance;
pub mod brush_collection;
pub mod brush_snapshot;
pub mod builtin_images;
pub mod captured_frame;
pub mod fastfile;
pub mod full_scene;
pub mod geometry_snapshot;
pub mod secondary_stream;
pub mod shader_bank;
pub mod vertex_signature;
