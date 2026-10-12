use crate::rendering::GameCamera;
use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};

pub(super) fn fit(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<GameCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let available = UVec2::new(window.physical_width(), window.physical_height());
    let Some(size) = [UVec2::new(16, 9), UVec2::new(4, 3)]
        .into_iter()
        .map(|ratio| ratio * (available.x / ratio.x).min(available.y / ratio.y))
        .filter(|size| size.x > 0 && size.y > 0)
        .max_by_key(|size| u64::from(size.x) * u64::from(size.y))
    else {
        return;
    };
    let position = (available - size) / 2;
    for mut camera in &mut cameras {
        if camera.viewport.as_ref().is_none_or(|viewport| {
            viewport.physical_size != size || viewport.physical_position != position
        }) {
            camera.viewport = Some(Viewport {
                physical_position: position,
                physical_size: size,
                ..default()
            });
        }
    }
}
