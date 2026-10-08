//! The RC fixes native extended-material bindless index-table lookup.
//! Use its material directly instead of retaining the old reversed-table adapter.
use super::OpaqueSurface;
use bevy::{pbr::ExtendedMaterial, prelude::StandardMaterial};

pub(in crate::rendering) type OpaqueMaterial = ExtendedMaterial<StandardMaterial, OpaqueSurface>;
