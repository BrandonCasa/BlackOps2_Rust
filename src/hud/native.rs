use bevy::{image::ImageSampler, prelude::*, render::render_resource::TextureFormat};
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

#[derive(Component, Default)]
pub(crate) struct Canvas;
#[derive(Component)]
pub(crate) struct Opacity(pub f32);
#[derive(Resource, Default)]
struct FadeCache(std::collections::HashMap<(Entity, u8), (Color, Color)>);
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
    ancestry: Query<(Option<&ChildOf>, Option<&Opacity>)>,
    mut images: Query<(Entity, &mut ImageNode)>,
    mut backgrounds: Query<(Entity, &mut BackgroundColor)>,
) {
    let factor = |entity| {
        let mut current = Some(entity);
        let mut alpha = 1.;
        while let Some(entity) = current {
            let Ok((parent, opacity)) = ancestry.get(entity) else {
                break;
            };
            if let Some(opacity) = opacity {
                alpha *= opacity.0;
            }
            current = parent.map(ChildOf::parent);
        }
        alpha
    };
    let mut apply = |entity, kind, color: &mut Color| {
        let alpha = factor(entity);
        if alpha == 1. && !cache.0.contains_key(&(entity, kind)) {
            return;
        }
        let (base, applied) = cache.0.entry((entity, kind)).or_insert((*color, *color));
        if *color != *applied {
            *base = *color;
        }
        *color = base.with_alpha(base.alpha() * alpha);
        *applied = *color;
    };
    for (entity, mut image) in &mut images {
        apply(entity, 0, &mut image.color);
    }
    for (entity, mut background) in &mut backgrounds {
        apply(entity, 1, &mut background.0);
    }
    cache.0.retain(|(entity, _), _| ancestry.contains(*entity));
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
    ImageNode::new(server.load(asset.path.clone()))
}

fn resize(
    cameras: Query<&Camera, With<crate::rendering::GameCamera>>,
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
    )>,
    glyphs: Query<(), With<Glyph>>,
) {
    let Some(game) = game else { return };
    if !labels
        .iter()
        .any(|(_, label, _, rendered, _)| rendered.is_none_or(|r| r.0 != *label))
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
    for (entity, label, mut node, rendered, children) in &mut labels {
        if rendered.is_some_and(|r| r.0 == *label) {
            continue;
        }
        let font_id = format!("ui.font.{}", label.style);
        let (asset, font) = fonts.get(font_id.as_str()).unwrap_or_else(|| {
            panic!("required HUD font {font_id} is missing from the UI catalog")
        });
        let texture = server.load(asset.path.clone());
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
            atlas.texture_descriptor.format = TextureFormat::Rgba8UnormSrgb;
            atlas.sampler = ImageSampler::linear();
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
