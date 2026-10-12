use bevy::render::settings::{Backends, WgpuSettings};

pub(crate) fn settings() -> WgpuSettings {
    if Backends::from_env().is_some_and(|backends| backends != Backends::VULKAN) {
        panic!("Origins requires Vulkan. Remove WGPU_BACKEND or set WGPU_BACKEND=vulkan.");
    }
    WgpuSettings {
        backends: Some(Backends::VULKAN),
        features: bevy::solari::SolariPlugins::required_wgpu_features(),
        force_fallback_adapter: false,
        ..Default::default()
    }
}
