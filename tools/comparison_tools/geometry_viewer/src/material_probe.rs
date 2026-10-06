//! Original-response material exercised on project-owned inputs only.
use super::*;
use bo2_rendering::tech4::{Tech4MaterialPlugin, synthetic_fixture};

pub fn run() -> Result<(), DiagnosticError> {
    let mut app = configured_app(false, None);
    app.add_plugins(Tech4MaterialPlugin);
    let report = gpu_probe::render(app, |world| {
        let (mesh, material, images) =
            synthetic_fixture(world).map_err(|_| DiagnosticError::Preservation)?;
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::IDENTITY,
        ));
        Ok(gpu_probe::ProbeScene {
            output: gpu_probe::ProbeOutput::default(),
            meshes: vec![mesh],
            images,
            camera: Transform::from_xyz(0.0, 3.0, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z),
            projection: Projection::Orthographic(OrthographicProjection {
                near: 0.1,
                far: 10.0,
                scaling_mode: bevy::camera::ScalingMode::Fixed {
                    width: 2.5,
                    height: 2.5,
                },
                ..OrthographicProjection::default_3d()
            }),
        })
    })?;
    let center = ((IMAGE_SIDE / 2 * IMAGE_SIDE + IMAGE_SIDE / 2) * 4) as usize;
    let sample = &report.pixels[center..center + 4];
    let foreground = report
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0)
        .count();
    if foreground == 0 || sample[..3] == [0, 0, 0] {
        return Err(DiagnosticError::BackgroundOnly);
    }
    // Fixture has constant ambient light, zero directional/probe response and no fog.
    // Compare the shader's linear result through the explicitly sRGB readback target.
    let ambient = (128.0_f32 / 255.0) / (1.0 + 1e-6);
    let expected = [128.0_f32, 64.0, 32.0].map(|channel| {
        let linear = channel / 255.0 * ambient.sqrt();
        let srgb = if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round() as u8
    });
    if (0..3).any(|channel| sample[channel].abs_diff(expected[channel]) > 2) || sample[3] != 255 {
        return Err(DiagnosticError::Preservation);
    }
    println!(
        "TECH4 synthetic GPU: updates={}, readback=512x512, foreground={foreground}, center={sample:?}; original fidelity unverified",
        report.updates
    );
    Ok(())
}
