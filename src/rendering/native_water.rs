//! Original Origins water, submitted after the native opaque scene resolve.
use std::{collections::BTreeMap, sync::Arc};

use super::native::{
    NativeWorldGeometry, NativeWorldObject, NativeWorldScene, NativeWorldState,
    PreparedNativeGeometry,
};
use super::native_effect::NativeEffectView;
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    camera::visibility::{self, VisibilityClass},
    ecs::{
        query::QueryItem,
        system::{SystemParamItem, lifetimeless::SRes},
    },
    mesh::VertexBufferLayout,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_resource::{
            binding_types::{sampler, texture_2d, texture_cube, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        sync_component::SyncComponent,
        texture::GpuImage,
        uniform::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        view::{RenderVisibleEntities, ViewDepthStencilTexture, ViewTarget},
    },
    shader::load_shader_library,
};
use serde::Deserialize;

#[derive(Clone, Copy, ShaderType)]
pub struct NativeWaterConstants {
    pub normal_position_scale: Vec4,
    pub lo_normal_scroll: Vec4,
    pub hi_normal_scroll: Vec4,
    pub normal_scroll_speed: Vec4,
    pub control0: Vec4,
    pub control1: Vec4,
    pub color_n: Vec4,
    pub color_s: Vec4,
    pub normal_scale: Vec4,
}

#[derive(Asset, TypePath, Clone)]
pub struct NativeWaterMaterial {
    pub constants: NativeWaterConstants,
    pub normal_map00: Handle<Image>,
    pub normal_sampler00: SamplerDescriptor<'static>,
    pub normal_map01: Handle<Image>,
    pub normal_sampler01: SamplerDescriptor<'static>,
    pub lightmap_secondary: Handle<Image>,
    pub lightmap_sampler: SamplerDescriptor<'static>,
    pub reflection_probe: Handle<Image>,
    pub reflection_sampler: SamplerDescriptor<'static>,
}

impl NativeWaterMaterial {
    pub fn images(&self) -> [&Handle<Image>; 4] {
        [
            &self.normal_map00,
            &self.normal_map01,
            &self.lightmap_secondary,
            &self.reflection_probe,
        ]
    }
}

/// Authored ComWorld data. 00430300 copies these values into GfxLight without
/// changing their source coordinate system or native-inch units.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryLightDefinition {
    #[serde(rename = "type")]
    pub kind: u8,
    pub use_cookie: i8,
    pub dir: Vec3,
    pub origin: Vec3,
    pub radius: f32,
    pub cos_half_fov_inner: f32,
    pub cos_half_fov_outer: f32,
    pub d_attenuation: f32,
    pub roundness: f32,
    pub diffuse_color: Vec4,
    pub falloff: Vec4,
    pub angle: Vec4,
    #[serde(rename = "aAbB")]
    pub a_ab_b: Vec4,
    pub cookie_control0: Vec4,
    pub cookie_control1: Vec4,
    pub cookie_control2: Vec4,
    pub def_name: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SunLightDefinition {
    #[serde(rename = "type")]
    pub kind: u8,
    pub dir: Vec3,
    pub diffuse_color: Vec4,
}

impl SunLightDefinition {
    /// 0043f830 rebuilds runtime GfxWorld.sunLight from these current original
    /// sun settings, using AngleVectors0074e8e0 and sunCd.rgb * sunCd.w.
    pub fn from_world_sun(angles: Vec3, sun_cd: Vec4) -> Self {
        let (sin_pitch, cos_pitch) = (angles.x * 0.017453292).sin_cos();
        let (sin_yaw, cos_yaw) = (angles.y * 0.017453292).sin_cos();
        Self {
            kind: 1,
            dir: Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, -sin_pitch),
            diffuse_color: (sun_cd.truncate() * sun_cd.w).extend(1.0),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightDefinition {
    pub attenuation_image: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaterTechniqueDefinition {
    pub slot: u8,
    pub name: String,
    pub flags: u16,
    pub pass_count: u16,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum WaterLightKind {
    Lit,
    Sun,
    Spot,
    Square,
    Round,
    Omni,
}

impl WaterLightKind {
    fn slot(self) -> u8 {
        match self {
            Self::Lit => 4,
            Self::Sun => 5,
            Self::Spot => 7,
            Self::Square => 9,
            Self::Round => 11,
            Self::Omni => 13,
        }
    }
    fn entry(self) -> &'static str {
        match self {
            Self::Lit => "fragment",
            Self::Sun => "fragment_sun",
            Self::Spot => "fragment_spot",
            Self::Square => "fragment_square",
            Self::Round => "fragment_round",
            Self::Omni => "fragment_omni",
        }
    }
    fn technique(self) -> &'static str {
        match self {
            Self::Lit => "pimp_technique_cod7water_2a4dbd75",
            Self::Sun => "pimp_technique_cod7water_40d5a312",
            Self::Spot => "pimp_technique_cod7water_c0e8f02",
            Self::Square => "pimp_technique_cod7water_ff47bd11",
            Self::Round => "pimp_technique_cod7water_75cf3849",
            Self::Omni => "pimp_technique_cod7water_d10386e4",
        }
    }
    fn cookie(self) -> bool {
        matches!(self, Self::Spot | Self::Square)
    }
}

#[derive(Clone)]
struct WaterLight {
    kind: WaterLightKind,
    source: PrimaryLightDefinition,
    cookie: Option<Handle<Image>>,
    // Original row-vector view/projection product, with zero eye translation.
    view_rows: [Vec4; 4],
    projection_rows: [Vec4; 4],
}

/// Construct only the actual primary lights referenced by water surfaces. The
/// indexed holes are unused source indices, never substitute lights.
pub struct NativeWaterLights {
    lights: Vec<Option<WaterLight>>,
}

impl NativeWaterLights {
    pub fn images(&self) -> impl Iterator<Item = &Handle<Image>> {
        self.lights
            .iter()
            .filter_map(|light| light.as_ref()?.cookie.as_ref())
    }

    pub fn from_source(
        primary: &[PrimaryLightDefinition],
        sun_index: u32,
        sun: &SunLightDefinition,
        definitions: &BTreeMap<String, LightDefinition>,
        techniques: &[WaterTechniqueDefinition],
        used_indices: impl IntoIterator<Item = u8>,
        server: &AssetServer,
    ) -> Result<Arc<Self>, String> {
        let mut lights = vec![None; primary.len()];
        for index in used_indices {
            let slot = lights.get_mut(usize::from(index)).ok_or_else(|| {
                format!("Water primaryLightIndex {index} exceeds ComWorld primaryLightCount")
            })?;
            if slot.is_some() {
                continue;
            }
            let mut source = primary[usize::from(index)].clone();
            if u32::from(index) == sun_index {
                if sun.kind != 1 {
                    return Err("Original GfxWorld sun override must have light type1".into());
                }
                source.kind = sun.kind;
                source.dir = sun.dir;
                source.diffuse_color = sun.diffuse_color;
            }
            // R_InitPrimaryLights00430300: type2 becomes square when roundness0,
            // or round when roundness1, equal authored axes and no cookie.
            let kind = match source.kind {
                0 => WaterLightKind::Lit,
                1 => WaterLightKind::Sun,
                2 if source.roundness == 0.0 => WaterLightKind::Square,
                2 if source.roundness == 1.0
                    && source.a_ab_b.x == source.a_ab_b.z
                    && source.a_ab_b.y == source.a_ab_b.w
                    && source.use_cookie == 0 =>
                {
                    WaterLightKind::Round
                }
                2 => WaterLightKind::Spot,
                3 => WaterLightKind::Square,
                4 => WaterLightKind::Round,
                5 => WaterLightKind::Omni,
                value => {
                    return Err(format!(
                        "Water primary light{index}: unsupported original light type{value}"
                    ));
                }
            };
            // 004658f0/00465b00/00477530 select the WORLD row of the technique
            // table. Shadow aliases are the same water shader payloads; they
            // have no shadow-image inputs. Require the selected native pass.
            let technique = techniques
                .iter()
                .find(|technique| technique.slot == kind.slot())
                .ok_or_else(|| {
                    format!(
                        "Water primary light{index}: missing original technique slot{}",
                        kind.slot()
                    )
                })?;
            if technique.name != kind.technique()
                || technique.pass_count != 1
                || technique.flags & 1 == 0
            {
                return Err(format!(
                    "Water technique {} requires recovered {} with its original single pass and resolvedPostSun flag",
                    technique.name,
                    kind.technique()
                ));
            }
            let cookie = if kind.cookie() {
                let name = source.def_name.as_ref().ok_or_else(|| {
                    format!("Water primary light{index}: missing original lightDef")
                })?;
                let definition = definitions.get(name).ok_or_else(|| {
                    format!("Water primary light{index}: missing lightDef {name}")
                })?;
                let image = definition.attenuation_image.as_ref().ok_or_else(|| {
                    format!(
                        "Water primary light{index}: lightDef {name} requires an attenuation image"
                    )
                })?;
                // 0047ea30 deliberately uses code sampler0x13, not the stored
                // lightDef samplerState. No cookie is bound for round/omni.
                Some(super::material::image(server, image))
            } else {
                None
            };
            let (view_rows, projection_rows) = if matches!(
                kind,
                WaterLightKind::Spot | WaterLightKind::Square | WaterLightKind::Round
            ) {
                source_projection(&source)
                    .map_err(|error| format!("Water primary light{index}: {error}"))?
            } else {
                ([Vec4::ZERO; 4], [Vec4::ZERO; 4])
            };
            *slot = Some(WaterLight {
                kind,
                source,
                cookie,
                view_rows,
                projection_rows,
            });
        }
        Ok(Arc::new(Self { lights }))
    }
}

fn multiply_rows(a: [Vec4; 4], b: [Vec4; 4]) -> [Vec4; 4] {
    a.map(|row| row.x * b[0] + row.y * b[1] + row.z * b[2] + row.w * b[3])
}

fn source_projection(light: &PrimaryLightDefinition) -> Result<([Vec4; 4], [Vec4; 4]), String> {
    // 0074b060 selects the axis least parallel to -dir, builds an orthonormal
    // view basis, then rotates XY by the authored angle.z (already radians).
    let forward = -light.dir;
    if !forward.is_finite() || (forward.length_squared() - 1.0).abs() > 0.001 {
        return Err("Original projected primary-light direction must be normalized".into());
    }
    let squared = forward * forward;
    let mut axis = usize::from(squared.y < squared.x);
    if squared.z < squared[axis] {
        axis = 2;
    }
    let mut up = forward * -forward[axis];
    up[axis] += 1.0;
    let length = up.length();
    up *= 1.0 / if length > 0.0 { length } else { 1.0 };
    let right = -up.cross(forward);
    let base = [
        Vec4::new(right.x, up.x, forward.x, 0.0),
        Vec4::new(right.y, up.y, forward.y, 0.0),
        Vec4::new(right.z, up.z, forward.z, 0.0),
        Vec4::W,
    ];
    let (sin, cos) = light.angle.z.sin_cos();
    let roll = [
        Vec4::new(cos, -sin, 0.0, 0.0),
        Vec4::new(sin, cos, 0.0, 0.0),
        Vec4::Z,
        Vec4::W,
    ];
    let view = multiply_rows(base, roll);
    // 0074b450: source light perspective uses falloff.x/y, with the native
    // near clamp. It is independent of the scene's reverse-Z projection.
    let near = light.falloff.x.max(0.001);
    let far = light.falloff.y;
    let cos_outer = light.cos_half_fov_outer;
    let scale = 1.0 / ((1.0 - cos_outer * cos_outer).sqrt() / cos_outer);
    let q = far / (far - near);
    let projection = [
        Vec4::new(scale, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(0.0, 0.0, q, 1.0),
        Vec4::new(0.0, 0.0, -q * near, 0.0),
    ];
    if !view.iter().chain(&projection).all(|row| row.is_finite()) {
        return Err(
            "Original projected primary-light view/projection contains nonfinite values".into(),
        );
    }
    Ok((view, projection))
}

// Compact copies of the source code constants actually read by the selected
// water program. Unread words in a variant are padding, not substitute inputs.
#[derive(Clone, Copy, Default, ShaderType)]
struct WaterLightConstants {
    position: Vec4,
    diffuse: Vec4,
    attenuation: Vec4,
    falloff_a: Vec4,
    falloff_b: Vec4,
    spot_matrix_rows: [Vec4; 3],
    spot_direction: Vec4,
    cookie_slide: Vec4,
    cone_control1: Vec4,
    cone_control2: Vec4,
}

impl WaterLight {
    fn constants(&self, eye: Vec3, time: f32, sun: &SunLightDefinition) -> WaterLightConstants {
        let light = &self.source;
        let mut out = WaterLightConstants {
            position: (light.origin - eye).extend(1.0),
            diffuse: light.diffuse_color.truncate().extend(1.0),
            attenuation: light.dir.extend(light.d_attenuation),
            ..default()
        };
        if self.kind == WaterLightKind::Sun {
            // 0043f830 rebuilds this source light from the current refdef sun;
            // the authored primary-light metadata is not its live RGB owner.
            out.position = sun.dir.extend(0.0);
            out.diffuse = sun.diffuse_color;
            return out;
        }
        if self.kind == WaterLightKind::Lit {
            return out;
        }
        // 0047ea30's original 1/4096 guard belongs to native light falloff
        // construction; the shader receives these exact reciprocals/offsets.
        let start = light.falloff.x;
        let end = light.falloff.y;
        let mut inner_start = light.falloff.z.max(start);
        if (inner_start - start).abs() <= 1.0 / 4096.0 {
            inner_start = start + 1.0 / 4096.0;
        }
        let mut inner_end = light.falloff.w.min(end);
        if (end - inner_end).abs() <= 1.0 / 4096.0 {
            inner_end = end - 1.0 / 4096.0;
        }
        let z = 1.0 / (inner_start - start);
        let w = 1.0 / (inner_end - end);
        out.falloff_a = Vec4::new(0.0, 0.0, z, w);
        out.falloff_b = Vec4::new(0.0, 0.0, -z * start, -w * end);
        if self.kind == WaterLightKind::Omni {
            return out;
        }
        let axes = light.a_ab_b;
        out.falloff_a.x = 1.0 / (axes.x - axes.y);
        out.falloff_a.y = 1.0 / (axes.z - axes.w);
        out.falloff_b.x = -out.falloff_a.x * axes.y;
        out.falloff_b.y = -out.falloff_a.y * axes.w;
        out.cone_control1 = axes.recip();
        let power = if light.roundness == 0.0 {
            1.0
        } else {
            2.0 / light.roundness
        };
        out.cone_control2 = Vec4::new(power, -1.0 / power, 1.0, light.roundness);
        let relative = light.origin - eye;
        let mut view = self.view_rows;
        view[3].x = -(view[0].x * relative.x + view[1].x * relative.y + view[2].x * relative.z);
        view[3].y = -(view[0].y * relative.x + view[1].y * relative.y + view[2].y * relative.z);
        view[3].z = -(view[0].z * relative.x + view[1].z * relative.y + view[2].z * relative.z);
        let matrix = multiply_rows(view, self.projection_rows);
        out.spot_matrix_rows =
            [0, 1, 2].map(|i| Vec4::new(matrix[0][i], matrix[1][i], matrix[2][i], matrix[3][i]));
        // Native sinf00a21231 / cosf00a21583, confirmed at their tiny-input
        // branches in the canonical exe. Cookie rotation/scroll uses game time.
        let c0 = light.cookie_control0;
        let c1 = light.cookie_control1;
        let c2 = light.cookie_control2;
        let (sin, cos) = (c2.z + c2.w * time).sin_cos();
        let u = ((c2.x - c2.x * cos) + c2.y * sin) * c0.z + c1.z * time + c1.x;
        let a = c0.w * sin + c0.y * c0.z * cos;
        let b = c0.w * cos + c0.y * -(c0.z * sin);
        let v = ((c2.y - c2.x * sin) - c2.y * cos) * c0.w + c1.w * time + c1.y + c0.y * u;
        out.spot_direction = Vec4::new(
            (c0.x * a + c0.z * cos) * 0.5,
            (c0.x * b - c0.z * sin) * 0.5,
            c0.x * v + u + 0.5,
            0.0,
        );
        out.cookie_slide = Vec4::new(a * 0.5, b * 0.5, v + 0.5, 0.0);
        out
    }
}

/// Camera owner supplies the original game clock in seconds and eye in native
/// source axes/inches. Frame owner supplies its code9 scene snapshot; code9/10
/// alias one physical image in native004549a0 ->00453ea0(target6,target7).
#[derive(Component, Clone)]
pub struct NativeWaterView {
    pub resolved_post_sun: Handle<Image>,
    pub game_time_seconds: f32,
    pub eye_position: Vec3,
    pub lights: Arc<NativeWaterLights>,
}

#[derive(Component, Clone, Copy, ShaderType)]
pub struct WaterScene {
    pub(crate) world: NativeWorldScene,
    game_time: Vec4,
    z_near: Vec4,
    render_target_size: Vec4,
}

#[derive(Component)]
pub struct ExtractedWaterView {
    resolved_post_sun: AssetId<Image>,
    float_z: AssetId<Image>,
    depth_sampler: SamplerDescriptor<'static>,
    eye: Vec3,
    time: f32,
    sun: SunLightDefinition,
    lights: Arc<NativeWaterLights>,
}

impl SyncComponent<RenderApp> for NativeWaterView {
    type Target = (ExtractedWaterView, WaterScene, PreparedWaterView);
}
impl ExtractComponent<RenderApp> for NativeWaterView {
    type QueryData = (
        &'static Self,
        &'static NativeWorldScene,
        &'static NativeEffectView,
    );
    type QueryFilter = ();
    type Out = (ExtractedWaterView, WaterScene);
    fn extract_component(
        (view, world, frame): QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        assert!(
            view.game_time_seconds.is_finite() && view.eye_position.is_finite(),
            "Water requires the original clock and source eye position"
        );
        Some((
            ExtractedWaterView {
                resolved_post_sun: view.resolved_post_sun.id(),
                float_z: frame.float_z.id(),
                depth_sampler: frame.float_z_sampler.clone(),
                eye: view.eye_position,
                time: view.game_time_seconds,
                sun: SunLightDefinition {
                    kind: 1,
                    dir: world.sun_position.truncate(),
                    diffuse_color: world.sun_diffuse,
                },
                lights: view.lights.clone(),
            },
            WaterScene {
                world: *world,
                game_time: super::native::shader_game_time(view.game_time_seconds),
                z_near: frame.z_near,
                render_target_size: frame.render_target_size,
            },
        ))
    }
}

#[derive(Component, Clone, ExtractComponent)]
#[extract_app(RenderApp)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<NativeWaterSurface>)]
pub struct NativeWaterSurface {
    pub geometry: Handle<NativeWorldGeometry>,
    pub material: Handle<NativeWaterMaterial>,
    pub state: NativeWorldState,
    pub primary_light_index: u8,
    pub primary_byte_offset: u64,
    pub first_index: u32,
    pub index_count: u32,
    /// Original draw-surface packed sort order, including primary-light index.
    pub sort_key: u64,
}

pub struct NativeWaterPlugin;
impl Plugin for NativeWaterPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<NativeWaterMaterial>().add_plugins((
            ExtractComponentPlugin::<NativeWaterView>::default(),
            ExtractComponentPlugin::<NativeWaterSurface>::default(),
            UniformComponentPlugin::<WaterScene>::default(),
            RenderAssetPlugin::<PreparedWaterMaterial, GpuImage>::default(),
        ));
        load_shader_library!(app, "native_water.wesl");
        embedded_asset!(app, "native_water_entry.wesl");
        let shader = load_embedded_asset!(app, "native_water_entry.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(WaterPipeline::new(shader))
            .init_resource::<SpecializedRenderPipelines<WaterPipeline>>()
            .init_resource::<WaterObjectGroup>()
            .add_systems(
                Render,
                prepare_water.in_set(RenderSystems::PrepareBindGroups),
            );
        // Frame owner schedules render_water between its scene snapshot and
        // second floatZ resolve, both before the ordinary transparent pass.
    }
}

#[derive(Resource)]
pub(crate) struct WaterPipeline {
    shader: Handle<Shader>,
    scene_layout: BindGroupLayoutDescriptor,
    object_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    light_layout: BindGroupLayoutDescriptor,
    cookie_layout: BindGroupLayoutDescriptor,
}

impl WaterPipeline {
    fn new(shader: Handle<Shader>) -> Self {
        Self {
            shader,
            scene_layout: BindGroupLayoutDescriptor::new(
                "T6 water frame",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        uniform_buffer::<WaterScene>(true),
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        sampler(SamplerBindingType::NonFiltering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            object_layout: BindGroupLayoutDescriptor::new(
                "T6 water object",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX,
                    uniform_buffer::<NativeWorldObject>(true),
                ),
            ),
            material_layout: BindGroupLayoutDescriptor::new(
                "T6 water material",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        uniform_buffer::<NativeWaterConstants>(false),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_cube(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            light_layout: BindGroupLayoutDescriptor::new(
                "T6 water primary light",
                &BindGroupLayoutEntries::single(
                    ShaderStages::FRAGMENT,
                    uniform_buffer::<WaterLightConstants>(false),
                ),
            ),
            cookie_layout: BindGroupLayoutDescriptor::new(
                "T6 water projected light cookie",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        uniform_buffer::<WaterLightConstants>(false),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct WaterPipelineKey {
    target: TextureFormat,
    samples: u32,
    state: NativeWorldState,
    light: WaterLightKind,
}

impl SpecializedRenderPipeline for WaterPipeline {
    type Key = WaterPipelineKey;
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // The DLSS main attachment stores the same encoded scene in float16;
        // its conversion to linear color runs after water and other native draws.
        assert!(
            matches!(
                key.target,
                TextureFormat::Rgba8Unorm | TextureFormat::Rgba16Float
            ),
            "Original water requires the encoded non-sRGB scene"
        );
        let mut layout = vec![
            self.scene_layout.clone(),
            self.object_layout.clone(),
            self.material_layout.clone(),
        ];
        if key.light != WaterLightKind::Lit {
            layout.push(if key.light.cookie() {
                self.cookie_layout.clone()
            } else {
                self.light_layout.clone()
            });
        }
        RenderPipelineDescriptor {
            label: Some("T6 original cod7water".into()),
            layout,
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![VertexBufferLayout {
                    array_stride: 36,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Unorm8x4,
                            offset: 16,
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Unorm16x2,
                            offset: 32,
                            shader_location: 2,
                        },
                    ],
                }],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some(key.light.entry().into()),
                targets: vec![Some(ColorTargetState {
                    format: key.target,
                    blend: None,
                    write_mask: key.state.color_write,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: key.state.front_face,
                cull_mode: key.state.cull_mode,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: super::frame::NATIVE_DEPTH_FORMAT,
                depth_write_enabled: Some(key.state.depth_write),
                depth_compare: Some(key.state.depth_compare),
                stencil: default(),
                bias: key.state.depth_bias(),
            }),
            multisample: MultisampleState {
                count: key.samples,
                ..default()
            },
            ..default()
        }
    }
}

pub(crate) struct PreparedWaterMaterial {
    group: BindGroup,
}
impl RenderAsset for PreparedWaterMaterial {
    type SourceAsset = NativeWaterMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<PipelineCache>,
        SRes<WaterPipeline>,
        SRes<RenderAssets<GpuImage>>,
    );
    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (device, queue, cache, pipeline, images): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let Some(gpu): Option<Vec<_>> = source
            .images()
            .iter()
            .map(|image| images.get(image.id()))
            .collect()
        else {
            return Err(PrepareAssetError::RetryNextUpdate(source));
        };
        assert!(
            gpu.iter()
                .all(|image| !image.texture_descriptor.format.is_srgb()),
            "Water requires original UNORM normal/lightmap/probe reads"
        );
        let samplers = [
            &source.normal_sampler00,
            &source.normal_sampler01,
            &source.lightmap_sampler,
            &source.reflection_sampler,
        ]
        .map(|sampler| device.create_sampler(sampler));
        let mut constants = UniformBuffer::from(source.constants);
        constants.write_buffer(device, queue);
        let group = device.create_bind_group(
            "T6 water material",
            &cache.get_bind_group_layout(&pipeline.material_layout),
            &BindGroupEntries::sequential((
                constants
                    .binding()
                    .expect("uploaded original water constants"),
                &gpu[0].texture_view,
                &samplers[0],
                &gpu[1].texture_view,
                &samplers[1],
                &gpu[2].texture_view,
                &samplers[2],
                &gpu[3].texture_view,
                &samplers[3],
            )),
        );
        Ok(Self { group })
    }
}

#[derive(Resource, Default)]
pub(crate) struct WaterObjectGroup(Option<(BufferId, BindGroup)>);

struct PreparedWaterLight {
    buffer: UniformBuffer<WaterLightConstants>,
    group: BindGroup,
    cookie: Option<TextureViewId>,
    eye: Vec3,
    time: f32,
}
struct WaterDraw {
    entity: Entity,
    pipeline: CachedRenderPipelineId,
    sort_key: u64,
}

#[derive(Component)]
pub struct PreparedWaterView {
    frame_buffer: BufferId,
    float_z: TextureViewId,
    resolved_post_sun: TextureViewId,
    group: BindGroup,
    lights: Vec<Option<PreparedWaterLight>>,
    draws: Vec<WaterDraw>,
    light_source: Arc<NativeWaterLights>,
}

impl PreparedWaterView {
    pub(crate) fn has_draws(&self) -> bool {
        !self.draws.is_empty()
    }
}

fn source_scene_sampler() -> SamplerDescriptor<'static> {
    // Default code sampler0x62,0043d230/00432ba0. Keep every source mip.
    SamplerDescriptor {
        label: Some("T6 resolvedPostSun code sampler0x62"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::Repeat,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    }
}

fn source_cookie_sampler() -> SamplerDescriptor<'static> {
    SamplerDescriptor {
        label: Some("T6 light cookie code sampler0x13"),
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        address_mode_w: AddressMode::Repeat,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        compare: None,
        anisotropy_clamp: 2,
        border_color: None,
    }
}

fn prepare_water(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<WaterPipeline>,
    mut variants: ResMut<SpecializedRenderPipelines<WaterPipeline>>,
    images: Res<RenderAssets<GpuImage>>,
    scenes: Res<ComponentUniforms<WaterScene>>,
    objects: Res<ComponentUniforms<NativeWorldObject>>,
    materials: Res<RenderAssets<PreparedWaterMaterial>>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    mut views: Query<(
        Entity,
        &ExtractedWaterView,
        &RenderVisibleEntities,
        &ViewTarget,
        &Msaa,
        Option<&mut PreparedWaterView>,
    )>,
    surfaces: Query<&NativeWaterSurface, With<NativeWorldObject>>,
    mut object_group: ResMut<WaterObjectGroup>,
) {
    let (Some(scene_buffer), Some(scene_binding), Some(object_buffer), Some(object_binding)) = (
        scenes.buffer(),
        scenes.binding(),
        objects.buffer(),
        objects.binding(),
    ) else {
        return;
    };
    if object_group
        .0
        .as_ref()
        .is_none_or(|(id, _)| *id != object_buffer.id())
    {
        object_group.0 = Some((
            object_buffer.id(),
            device.create_bind_group(
                "T6 water object",
                &cache.get_bind_group_layout(&pipeline.object_layout),
                &BindGroupEntries::single(object_binding),
            ),
        ));
    }
    for (entity, view, visible, target, msaa, previous) in &mut views {
        let (Some(depth), Some(scene)) =
            (images.get(view.float_z), images.get(view.resolved_post_sun))
        else {
            continue;
        };
        assert!(
            view.depth_sampler.mag_filter == FilterMode::Nearest
                && view.depth_sampler.min_filter == FilterMode::Nearest
                && view.depth_sampler.mipmap_filter == MipmapFilterMode::Nearest
                && view.depth_sampler.compare.is_none(),
            "Water requires original point code sampler0x61 for floatZ"
        );
        assert_eq!(
            scene.texture_descriptor.format,
            target.main_texture_format(),
            "Water code9 snapshot must match the encoded main attachment"
        );
        assert_eq!(
            scene.texture_descriptor.sample_count, 1,
            "Water code9 requires a resolved single-sample image"
        );
        assert_eq!(
            depth.texture_descriptor.size, scene.texture_descriptor.size,
            "Water depth and refraction must match source render-target size"
        );
        let mut owned = None;
        let prepared = if let Some(previous) = previous {
            previous.into_inner()
        } else {
            let depth_sampler = device.create_sampler(&view.depth_sampler);
            let scene_sampler = device.create_sampler(&source_scene_sampler());
            owned = Some(PreparedWaterView {
                frame_buffer: scene_buffer.id(),
                float_z: depth.texture_view.id(),
                resolved_post_sun: scene.texture_view.id(),
                group: device.create_bind_group(
                    "T6 water frame",
                    &cache.get_bind_group_layout(&pipeline.scene_layout),
                    &BindGroupEntries::sequential((
                        scene_binding.clone(),
                        &depth.texture_view,
                        &depth_sampler,
                        &scene.texture_view,
                        &scene_sampler,
                    )),
                ),
                lights: (0..view.lights.lights.len()).map(|_| None).collect(),
                draws: Vec::new(),
                light_source: view.lights.clone(),
            });
            owned.as_mut().expect("new water view")
        };
        if prepared.frame_buffer != scene_buffer.id()
            || prepared.float_z != depth.texture_view.id()
            || prepared.resolved_post_sun != scene.texture_view.id()
        {
            let depth_sampler = device.create_sampler(&view.depth_sampler);
            let scene_sampler = device.create_sampler(&source_scene_sampler());
            prepared.group = device.create_bind_group(
                "T6 water frame",
                &cache.get_bind_group_layout(&pipeline.scene_layout),
                &BindGroupEntries::sequential((
                    scene_binding.clone(),
                    &depth.texture_view,
                    &depth_sampler,
                    &scene.texture_view,
                    &scene_sampler,
                )),
            );
            prepared.frame_buffer = scene_buffer.id();
            prepared.float_z = depth.texture_view.id();
            prepared.resolved_post_sun = scene.texture_view.id();
        }
        if !Arc::ptr_eq(&prepared.light_source, &view.lights) {
            prepared.lights = (0..view.lights.lights.len()).map(|_| None).collect();
            prepared.light_source = view.lights.clone();
        }
        prepared.draws.clear();
        if let Some(visible) = visible.get::<NativeWaterSurface>() {
            for (&entity, _) in visible.iter_visible() {
                let Ok(surface) = surfaces.get(entity) else {
                    continue;
                };
                if surface.index_count == 0
                    || geometries.get(surface.geometry.id()).is_none()
                    || materials.get(surface.material.id()).is_none()
                {
                    continue;
                }
                let index = usize::from(surface.primary_light_index);
                let light = view
                    .lights
                    .lights
                    .get(index)
                    .and_then(Option::as_ref)
                    .expect("owned source water primary light");
                if light.kind != WaterLightKind::Lit {
                    let cookie = match &light.cookie {
                        Some(cookie) => {
                            let Some(image) = images.get(cookie.id()) else {
                                continue;
                            };
                            Some(image)
                        }
                        None => None,
                    };
                    let existing = &mut prepared.lights[index];
                    if existing
                        .as_ref()
                        .is_none_or(|old| old.cookie != cookie.map(|image| image.texture_view.id()))
                    {
                        let mut buffer =
                            UniformBuffer::from(light.constants(view.eye, view.time, &view.sun));
                        buffer.write_buffer(&device, &queue);
                        let group = if let Some(cookie) = cookie {
                            let sampler = device.create_sampler(&source_cookie_sampler());
                            device.create_bind_group(
                                "T6 water light and cookie",
                                &cache.get_bind_group_layout(&pipeline.cookie_layout),
                                &BindGroupEntries::sequential((
                                    buffer.binding().expect("uploaded light constants"),
                                    &cookie.texture_view,
                                    &sampler,
                                )),
                            )
                        } else {
                            device.create_bind_group(
                                "T6 water light",
                                &cache.get_bind_group_layout(&pipeline.light_layout),
                                &BindGroupEntries::single(
                                    buffer.binding().expect("uploaded light constants"),
                                ),
                            )
                        };
                        *existing = Some(PreparedWaterLight {
                            buffer,
                            group,
                            cookie: cookie.map(|image| image.texture_view.id()),
                            eye: view.eye,
                            time: view.time,
                        });
                    } else if existing.as_ref().is_some_and(|old| {
                        if light.kind == WaterLightKind::Sun {
                            old.buffer.get().position != view.sun.dir.extend(0.)
                                || old.buffer.get().diffuse != view.sun.diffuse_color
                        } else {
                            old.eye != view.eye || old.time != view.time
                        }
                    }) {
                        let existing = existing.as_mut().expect("existing water light");
                        existing
                            .buffer
                            .set(light.constants(view.eye, view.time, &view.sun));
                        existing.buffer.write_buffer(&device, &queue);
                        existing.eye = view.eye;
                        existing.time = view.time;
                    }
                }
                let pipeline_id = variants.specialize(
                    &cache,
                    &pipeline,
                    WaterPipelineKey {
                        target: target.main_texture_format(),
                        samples: msaa.samples(),
                        state: surface.state,
                        light: light.kind,
                    },
                );
                prepared.draws.push(WaterDraw {
                    entity,
                    pipeline: pipeline_id,
                    sort_key: surface.sort_key,
                });
            }
        }
        prepared.draws.sort_unstable_by_key(|draw| draw.sort_key);
        if let Some(owned) = owned {
            commands.entity(entity).insert(owned);
        }
    }
}

/// The frame owner places this after code9 resolve and before the second
/// floatZ resolve. 00424ff0/00468f50: source sortKey16 water is opaque post-
/// resolve geometry; it writes depth before the ordinary translucent effects.
pub(crate) fn render_water(
    view: ViewQuery<(
        &ViewTarget,
        &ViewDepthStencilTexture,
        &ExtractedCamera,
        &DynamicUniformIndex<WaterScene>,
        &PreparedWaterView,
        Option<&bevy::camera::MainPassResolutionOverride>,
    )>,
    cache: Res<PipelineCache>,
    objects: Res<WaterObjectGroup>,
    materials: Res<RenderAssets<PreparedWaterMaterial>>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    surfaces: Query<(&NativeWaterSurface, &DynamicUniformIndex<NativeWorldObject>)>,
    mut ctx: RenderContext,
) {
    let (target, depth, camera, scene, prepared, resolution) = view.into_inner();
    let Some((_, object_group)) = &objects.0 else {
        return;
    };
    if prepared.draws.is_empty() {
        return;
    }
    let attachments = [Some(target.get_color_attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("T6 water opaque post-resolve"),
        color_attachments: &attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_bind_group(0, &prepared.group, &[scene.index()]);
    if let Some(viewport) =
        bevy::camera::Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution)
    {
        pass.set_camera_viewport(&viewport);
    }
    for draw in &prepared.draws {
        let (Some(pipeline), Ok((surface, object))) = (
            cache.get_render_pipeline(draw.pipeline),
            surfaces.get(draw.entity),
        ) else {
            continue;
        };
        let (Some(material), Some(geometry)) = (
            materials.get(surface.material.id()),
            geometries.get(surface.geometry.id()),
        ) else {
            continue;
        };
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(1, object_group, &[object.index()]);
        pass.set_bind_group(2, &material.group, &[]);
        if let Some(light) = &prepared.lights[usize::from(surface.primary_light_index)] {
            pass.set_bind_group(3, &light.group, &[]);
        }
        pass.set_vertex_buffer(0, geometry.primary.slice(surface.primary_byte_offset..));
        pass.set_index_buffer(geometry.indices.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(
            surface.first_index..surface.first_index + surface.index_count,
            0,
            0..1,
        );
    }
}
