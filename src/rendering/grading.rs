use super::profile::{ColorGrade, VisualProfile};
use bevy::{core_pipeline::tonemapping::TonemappingLuts, prelude::*};

pub(crate) fn install(
    images: &mut Assets<Image>,
    luts: &mut TonemappingLuts,
    profile: &VisualProfile,
) {
    let mut graded = images
        .get(&luts.agx)
        .expect("Bevy's enabled AgX display LUT must exist")
        .clone();
    let size = graded.texture_descriptor.size;
    assert_eq!(
        [size.width, size.height, size.depth_or_array_layers],
        [32; 3]
    );
    graded.texture_descriptor.label = Some("Origins graded AgX LUT");
    for z in 0..32 {
        for y in 0..32 {
            for x in 0..32 {
                let original = graded
                    .get_color_at_3d(x, y, z)
                    .expect("Native AgX LUT has accessible float texels");
                graded
                    .set_color_at_3d(x, y, z, grade(original, &profile.color_grade))
                    .expect("Native AgX LUT accepts float texels");
            }
        }
    }
    luts.agx = images.add(graded);
}

fn grade(original: Color, settings: &ColorGrade) -> Color {
    let display = original.to_srgba();
    let rgb = Vec3::new(display.red, display.green, display.blue).clamp(Vec3::ZERO, Vec3::ONE);
    let luma = luminance(rgb);
    const PIVOT: f32 = 0.46;
    let curved = if luma > 0. && luma < 1. {
        let odds = PIVOT / (1. - PIVOT);
        let value = odds * ((luma / (1. - luma)) / odds).powf(settings.contrast.clamp(0.5, 1.5));
        value / (1. + value)
    } else {
        luma
    };
    let contrasted = rgb * (curved / luma.max(1e-6));
    let saturation = (rgb.max_element() - rgb.min_element()) / rgb.max_element().max(1e-6);
    let vibrance = 1. + (settings.vibrance.clamp(0., 2.) - 1.) * (1. - saturation);
    let vibrant = Vec3::splat(curved) + (contrasted - Vec3::splat(curved)) * vibrance;
    let shadow_weight = smooth(1. - luma * 2.);
    let highlight_weight = smooth(luma * 2. - 1.);
    let tint = Vec3::ONE
        + (Vec3::from_array(settings.shadow_tint) - Vec3::ONE) * shadow_weight
        + (Vec3::from_array(settings.highlight_tint) - Vec3::ONE) * highlight_weight;
    let graded = fit_gamut(vibrant * tint);
    let result = rgb.lerp(graded, settings.strength.clamp(0., 1.));
    Color::srgba(result.x, result.y, result.z, display.alpha)
}

fn luminance(rgb: Vec3) -> f32 {
    rgb.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

fn smooth(value: f32) -> f32 {
    let value = value.clamp(0., 1.);
    value * value * (3. - 2. * value)
}

fn fit_gamut(rgb: Vec3) -> Vec3 {
    let luma = luminance(rgb).clamp(0., 1.);
    let mut scale = 1_f32;
    for channel in rgb.to_array() {
        if channel > 1. {
            scale = scale.min((1. - luma) / (channel - luma));
        } else if channel < 0. {
            scale = scale.min(luma / (luma - channel));
        }
    }
    (Vec3::splat(luma) + (rgb - Vec3::splat(luma)) * scale).clamp(Vec3::ZERO, Vec3::ONE)
}
