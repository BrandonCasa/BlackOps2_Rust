use super::OpaqueSurface;
use bevy::{pbr::ExtendedMaterial, prelude::StandardMaterial};

pub(in crate::rendering) type OpaqueMaterial = ExtendedMaterial<StandardMaterial, OpaqueSurface>;
