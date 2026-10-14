use bevy::{
    asset::LoadContext,
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, CompareFunction,
        RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use serde::Deserialize;
use std::io;

#[derive(Deserialize)]
pub(super) struct Texture {
    uri: String,
    sampler: u32,
    color_space: String,
}

#[derive(Deserialize)]
pub(super) struct Definition {
    sprite: Texture,
    grime: Texture,
    intensity: f32,
    scale: f32,
    consumer: String,
    red_producer: String,
}

pub(super) struct Prepared {
    pub sprite: Handle<Image>,
    pub grime: Handle<Image>,
    intensity: f32,
    scale: f32,
}

impl Definition {
    pub(super) fn load(self, context: &mut LoadContext<'_>) -> io::Result<Prepared> {
        if self.consumer != "native_cached_superflare"
            || self.red_producer != "native_cached_visible_ordinal"
            || self.intensity != 0.5
            || self.scale != 2.5
        {
            return Err(io::Error::other(
                "Invalid native cached superflare controls",
            ));
        }
        let mut load = |texture: Texture| -> io::Result<Handle<Image>> {
            crate::origins::validate_asset_path(&texture.uri).map_err(io::Error::other)?;
            if texture.sampler != 226 || texture.color_space != "native_encoded" {
                return Err(io::Error::other(
                    "Invalid native superflare sampler or transfer",
                ));
            }
            Ok(context
                .load_builder()
                .with_settings(|settings: &mut ImageLoaderSettings| {
                    settings.is_srgb = false;
                    settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::ClampToEdge,
                        address_mode_v: ImageAddressMode::ClampToEdge,
                        address_mode_w: ImageAddressMode::ClampToEdge,
                        lod_max_clamp: 0.,
                        ..ImageSamplerDescriptor::linear()
                    });
                })
                .load(texture.uri))
        };
        Ok(Prepared {
            sprite: load(self.sprite)?,
            grime: load(self.grime)?,
            intensity: self.intensity,
            scale: self.scale,
        })
    }
}

#[derive(Clone, ShaderType)]
struct Controls {
    flare: Vec4,
    atlas: Vec4,
    draw: UVec4,
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub(super) struct SuperflareMaterial {
    #[uniform(0)]
    controls: Controls,
    #[texture(1)]
    #[sampler(2)]
    #[dependency]
    sprite: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    #[dependency]
    grime: Handle<Image>,
    curve_alpha: f32,
}

impl Prepared {
    pub(super) fn material(&self) -> SuperflareMaterial {
        SuperflareMaterial {
            controls: Controls {
                flare: Vec4::new(self.intensity, self.scale, 0., 0.),
                atlas: Vec4::new(1., 1., 0., 0.),
                draw: UVec4::ZERO,
            },
            sprite: self.sprite.clone(),
            grime: self.grime.clone(),
            curve_alpha: 0.,
        }
    }
}

impl SuperflareMaterial {
    pub(super) fn sample(&mut self, alpha: f32, atlas: Vec4) {
        self.curve_alpha = alpha;
        self.controls.atlas = atlas;
    }
    pub(super) fn curve_alpha(&self) -> f32 {
        self.curve_alpha
    }
    pub(super) fn admit(&mut self, alpha: u8, ordinal: Option<u8>) {
        self.controls.flare.z = alpha as f32 / 255.;
        self.controls.draw.x = u32::from(ordinal.is_some());
        if let Some(ordinal) = ordinal {
            self.controls.flare.w = ordinal as f32 / 255.;
        }
    }
}

pub(super) fn distance_fade(distance: f32, near: [f32; 2], far: [f32; 2]) -> f32 {
    let declining = |range: [f32; 2]| {
        let distance = distance - range[0];
        if distance < 0. {
            1.
        } else if distance < range[1] {
            1. - distance / range[1]
        } else {
            0.
        }
    };
    let first = if near[1] == 0. { 1. } else { declining(near) };
    let second = if far[1] == 0. {
        1.
    } else {
        1. - declining(far)
    };
    first.min(second)
}

impl bevy::pbr::Material for SuperflareMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://origins/native_fx/superflare.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://origins/native_fx/superflare.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn enable_oit() -> bool {
        false
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_compare = Some(CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        if let Some(fragment) = &mut descriptor.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
        Ok(())
    }
}

pub(super) fn install(app: &mut App) {
    bevy::shader::load_shader_library!(app, "superflare.wesl");
    app.add_plugins(MaterialPlugin::<SuperflareMaterial>::default());
}
