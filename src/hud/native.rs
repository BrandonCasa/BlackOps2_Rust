use bevy::{
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    prelude::*,
    render::render_resource::TextureFormat,
};
use std::collections::BTreeMap;

#[derive(Component, Clone, PartialEq)]
pub(crate) struct NativeText {
    pub value: String,
    pub size: f32,
    pub color: Color,
    pub style: &'static str,
    pub align: Justify,
}
impl NativeText {
    pub fn new(value: impl Into<String>, size: f32) -> Self {
        Self {
            value: value.into(),
            size,
            color: Color::WHITE,
            style: "small",
            align: Justify::Left,
        }
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    pub fn centered(mut self) -> Self {
        self.align = Justify::Center;
        self
    }
}

/// UI whose colors carry the original numeric RGB through Bevy extraction.
#[derive(Component, Default)]
pub(crate) struct NativeUiRoot;
#[derive(Component, Default)]
#[require(NativeUiRoot)]
pub(crate) struct Canvas;
#[derive(Component)]
pub(crate) struct Opacity(pub f32);
#[derive(Resource, Default)]
struct FadeCache(std::collections::HashMap<(Entity, u8), (Vec4, Vec4)>);
#[derive(Component)]
pub(crate) struct SafeAnchor {
    pub x: f32,
    pub side: f32,
}
impl SafeAnchor {
    pub fn left(x: f32) -> Self {
        Self { x, side: -1. }
    }
    pub fn right(x: f32) -> Self {
        Self { x, side: 1. }
    }
}
#[derive(Component)]
struct Glyph;
#[derive(Component)]
struct Rendered(NativeText);
#[derive(Component)]
struct FontAtlas {
    style: &'static str,
    image: Handle<Image>,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<FadeCache>().add_systems(
        PostUpdate,
        (resize, render, fade)
            .chain()
            .before(bevy::ui::UiSystems::Layout),
    );
}

fn fade(
    mut cache: ResMut<FadeCache>,
    ancestry: Query<(Option<&ChildOf>, Option<&Opacity>, Has<NativeUiRoot>)>,
    mut images: Query<(Entity, &mut ImageNode)>,
    mut backgrounds: Query<(Entity, &mut BackgroundColor)>,
    mut borders: Query<(Entity, &mut BorderColor)>,
) {
    let factor = |entity| {
        let mut current = Some(entity);
        let mut alpha = 1.;
        let mut native = false;
        while let Some(entity) = current {
            let Ok((parent, opacity, root)) = ancestry.get(entity) else {
                break;
            };
            native |= root;
            if let Some(opacity) = opacity {
                alpha *= opacity.0;
            }
            current = parent.map(ChildOf::parent);
        }
        native.then_some(alpha)
    };
    let mut apply = |entity, kind, color: Color| {
        let alpha = factor(entity)?;
        let input = color.to_linear().to_vec4();
        let (base, applied) = cache.0.entry((entity, kind)).or_insert((input, input));
        // Animation often changes only alpha on the previous rendered tint.
        // Retain unchanged authored channels before the new byte conversion.
        for channel in 0..4 {
            if input[channel] != applied[channel] {
                base[channel] = input[channel];
            }
        }
        let mut value = *base;
        value.w *= alpha;
        let packed = packed_rgba(value);
        *applied = packed;
        (input != packed).then(|| Color::linear_rgba(packed.x, packed.y, packed.z, packed.w))
    };
    for (entity, mut image) in &mut images {
        if let Some(color) = apply(entity, 0, image.color) {
            image.color = color;
        }
    }
    for (entity, mut background) in &mut backgrounds {
        if let Some(color) = apply(entity, 1, background.0) {
            background.0 = color;
        }
    }
    for (entity, mut border) in &mut borders {
        for (kind, color) in [
            (2, border.top),
            (3, border.right),
            (4, border.bottom),
            (5, border.left),
        ] {
            if let Some(color) = apply(entity, kind, color) {
                match kind {
                    2 => border.top = color,
                    3 => border.right = color,
                    4 => border.bottom = color,
                    _ => border.left = color,
                }
            }
        }
    }
    cache.0.retain(|(entity, _), _| ancestry.contains(*entity));
}

/// 00426e30: staged f32 operations, then CVTPS2DQ nearest-even and byte clamp.
/// This happens after inherited opacity; it is not an RGB transfer function.
fn packed_channel(value: f32) -> u8 {
    let scaled = value * 255.;
    let biased = scaled + 0.5;
    let rounded = (biased - f32::from_bits(0x3eff_fffd)).round_ties_even();
    // CVTPS2DQ returns INT_MIN for nonfinite/out-of-range inputs.
    let integer = if !rounded.is_finite() || !(-2147483648. ..2147483648.).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    };
    integer.clamp(0, 255) as u8
}
pub(crate) fn packed_rgba(value: Vec4) -> Vec4 {
    Vec4::from_array(
        value
            .to_array()
            .map(|channel| f32::from(packed_channel(channel)) / 255.),
    )
}

pub(crate) fn rect(x: f32, y: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        width: px(width),
        height: px(height),
        ..default()
    }
}

pub(crate) fn image(
    game: &crate::assets::GameDefinition,
    server: &AssetServer,
    name: &str,
) -> ImageNode {
    let id = format!("ui.{name}");
    let asset = game
        .content
        .ui_assets
        .iter()
        .find(|asset| asset.id.0 == id)
        .unwrap_or_else(|| panic!("required HUD image {id} is missing from the UI catalog"));
    ImageNode::new(load_image(server, asset))
}

pub(crate) fn load_image(
    server: &AssetServer,
    asset: &crate::origins::content::UiAsset,
) -> Handle<Image> {
    let Some(source) = &asset.sampler else {
        // Unresolved legacy menu/weapon-image mappings retain their current
        // loading behavior until their original material inputs are exported.
        return server.load(asset.path.clone());
    };
    let material = asset
        .material
        .as_deref()
        .expect("UI sampler requires its original material identity");
    assert!(
        !matches!(
            material,
            "hud_icon_m249saw_mounted"
                | "hud_mp_firerate_auto"
                | "hud_mp_firerate_bolt"
                | "hud_mp_firerate_pump"
                | "zm_tm_wth_dog"
                | "fonts/720/gamefonts_pc_glow"
        ),
        "Add original UiMaterial shader/blend for {material} before using it"
    );
    let filter = match source.filter.as_str() {
        "nearest" => ImageFilterMode::Nearest,
        "linear" => ImageFilterMode::Linear,
        other => panic!("Unsupported original UI filter {other} for {material}"),
    };
    let (mip, max_lod) = match source.mip_map.as_str() {
        "disabled" => (ImageFilterMode::Nearest, 0.),
        "nearest" => (ImageFilterMode::Nearest, 32.),
        "linear" => (ImageFilterMode::Linear, 32.),
        other => panic!("Unsupported original UI mip sampler {other} for {material}"),
    };
    let addressing = |clamp| {
        if clamp {
            ImageAddressMode::ClampToEdge
        } else {
            ImageAddressMode::Repeat
        }
    };
    let sampler = ImageSamplerDescriptor {
        address_mode_u: addressing(source.clamp_u),
        address_mode_v: addressing(source.clamp_v),
        address_mode_w: addressing(source.clamp_w),
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: mip,
        lod_max_clamp: max_lod,
        ..default()
    };
    server
        .load_builder()
        .with_settings(move |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            settings.sampler = ImageSampler::Descriptor(sampler.clone());
        })
        .load(asset.path.clone())
}

fn resize(
    cameras: Query<&Camera, With<crate::rendering::display::GameUiCamera>>,
    scale: Res<UiScale>,
    mut roots: Query<(&mut Node, &mut UiTransform), With<Canvas>>,
    mut anchors: Query<(&SafeAnchor, &mut Node), Without<Canvas>>,
) {
    let Some(size) = cameras
        .single()
        .ok()
        .and_then(Camera::logical_viewport_size)
    else {
        return;
    };
    let size = size / scale.0;
    let factor = size.y / 720.;
    for (mut node, mut transform) in &mut roots {
        let left = (size.x - 1280.) * 0.5;
        let top = (size.y - 720.) * 0.5;
        *node = rect(left, top, 1280., 720.);
        transform.scale = Vec2::splat(factor);
    }
    let offset = (size.x / factor - 1280.) * 0.5;
    for (anchor, mut node) in &mut anchors {
        node.left = px(anchor.x + anchor.side * offset);
    }
}

fn render(
    mut commands: Commands,
    game: Option<Res<crate::assets::GameDefinition>>,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut labels: Query<(
        Entity,
        &NativeText,
        &mut Node,
        Option<&Rendered>,
        Option<&Children>,
        Option<&FontAtlas>,
    )>,
    glyphs: Query<(), With<Glyph>>,
) {
    let Some(game) = game else { return };
    if !labels
        .iter()
        .any(|(_, label, _, rendered, _, _)| rendered.is_none_or(|r| r.0 != *label))
    {
        return;
    }
    let fonts: BTreeMap<_, _> = game
        .content
        .ui_assets
        .iter()
        .filter_map(|asset| {
            asset
                .font
                .as_ref()
                .map(|font| (asset.id.0.as_str(), (asset, font)))
        })
        .collect();
    for (entity, label, mut node, rendered, children, atlas) in &mut labels {
        if rendered.is_some_and(|r| r.0 == *label) {
            continue;
        }
        let font_id = format!("ui.font.{}", label.style);
        let (asset, font) = fonts.get(font_id.as_str()).unwrap_or_else(|| {
            panic!("required HUD font {font_id} is missing from the UI catalog")
        });
        let texture = if let Some(atlas) = atlas.filter(|atlas| atlas.style == label.style) {
            atlas.image.clone()
        } else {
            let image = load_image(&server, asset);
            // Keep the atlas alive while it loads, before glyphs can hold it.
            commands.entity(entity).insert(FontAtlas {
                style: label.style,
                image: image.clone(),
            });
            image
        };
        let Some(atlas) = images.get(&texture) else {
            continue;
        };
        if atlas.texture_descriptor.format == TextureFormat::R8Unorm {
            let mut atlas = images.get_mut(&texture).unwrap();
            // The source font is DDS A8. Bevy loads A8 as R8, while the original
            // alphafont shader samples coverage from alpha and emits white RGB.
            let coverage = atlas
                .data
                .take()
                .expect("required font atlas has no CPU image data");
            atlas.data = Some(
                coverage
                    .into_iter()
                    .flat_map(|alpha| [255, 255, 255, alpha])
                    .collect(),
            );
            atlas.texture_descriptor.format = TextureFormat::Rgba8Unorm;
        }
        if let Some(children) = children {
            for child in children.iter().filter(|child| glyphs.contains(*child)) {
                commands.entity(child).despawn();
            }
        }
        let metrics: BTreeMap<_, _> = font
            .glyphs
            .iter()
            .filter_map(|glyph| char::from_u32(glyph.codepoint).map(|letter| (letter, glyph)))
            .collect();
        let factor = label.size / font.height;
        let advance = |text: &str| {
            text.chars()
                .filter_map(|c| metrics.get(&c).or_else(|| metrics.get(&'?')))
                .map(|g| g.advance * factor)
                .sum::<f32>()
        };
        let mut lines = Vec::<String>::new();
        for source in label.value.lines() {
            if let Val::Px(width) = node.width
                && advance(source) > width
            {
                let mut line = String::new();
                for word in source.split_whitespace() {
                    let next = if line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{line} {word}")
                    };
                    if advance(&next) > width && !line.is_empty() {
                        lines.push(std::mem::take(&mut line));
                        line = word.into();
                    } else {
                        line = next;
                    }
                }
                lines.push(line);
            } else {
                lines.push(source.into());
            }
        }
        let widths: Vec<f32> = lines.iter().map(|line| advance(line)).collect();
        let width = if let Val::Px(width) = node.width {
            width
        } else {
            widths.iter().copied().fold(0., f32::max)
        };
        if node.width == Val::Auto {
            node.width = px(width);
        }
        if node.height == Val::Auto {
            node.height = px(label.size * lines.len().max(1) as f32);
        }
        commands.entity(entity).with_children(|parent| {
            for (line, text) in lines.iter().enumerate() {
                let mut x = match label.align {
                    Justify::Center => (width - widths[line]) * 0.5,
                    Justify::Right => width - widths[line],
                    _ => 0.,
                };
                for letter in text.chars() {
                    let Some(glyph) = metrics.get(&letter).or_else(|| metrics.get(&'?')) else {
                        continue;
                    };
                    if glyph.size[0] > 0. && glyph.size[1] > 0. {
                        let left = x + glyph.offset[0] * factor;
                        let top =
                            (font.height + glyph.offset[1]) * factor + line as f32 * label.size;
                        parent.spawn((
                            Glyph,
                            Pickable::IGNORE,
                            rect(left, top, glyph.size[0] * factor, glyph.size[1] * factor),
                            ImageNode {
                                image: texture.clone(),
                                color: label.color,
                                rect: Some(Rect::new(
                                    glyph.rect[0],
                                    glyph.rect[1],
                                    glyph.rect[2],
                                    glyph.rect[3],
                                )),
                                ..default()
                            },
                        ));
                    }
                    x += glyph.advance * factor;
                }
            }
        });
        commands
            .entity(entity)
            .insert((Rendered(label.clone()), Pickable::IGNORE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_ui_byte_packing_uses_staged_nearest_even_and_clamps() {
        assert_eq!(
            [0.1, 0.3, 0.5, 0.7, 0.9].map(packed_channel),
            [26, 76, 128, 178, 230]
        );
        assert_eq!([-1., 0., 1., 2.].map(packed_channel), [0, 0, 255, 255]);
        assert_eq!(packed_channel(f32::NAN), 0);
        assert_eq!(packed_channel(f32::INFINITY), 0);
        let color = packed_rgba(Vec4::new(0.5, 0.3, 0.7, 0.25));
        assert_eq!(
            Color::linear_rgba(color.x, color.y, color.z, color.w)
                .to_linear()
                .to_vec4(),
            color
        );
    }
    #[test]
    fn opacity_is_combined_before_source_alpha_byte_packing() {
        let mut app = App::new();
        app.init_resource::<FadeCache>().add_systems(Update, fade);
        let root = app.world_mut().spawn((NativeUiRoot, Opacity(0.5))).id();
        let image = app
            .world_mut()
            .spawn((
                ChildOf(root),
                ImageNode::solid_color(Color::linear_rgba(0.3, 0.7, 0.9, 0.01)),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<ImageNode>(image).unwrap().color.alpha(),
            1. / 255.
        );
        app.world_mut().get_mut::<Opacity>(root).unwrap().0 = 1.;
        app.update();
        assert_eq!(
            app.world().get::<ImageNode>(image).unwrap().color.alpha(),
            3. / 255.
        );
    }
}
