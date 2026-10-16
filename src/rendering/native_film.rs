//! Ordinary native bloom and film. The camera owns vision selection/transitions;
//! this owner consumes its interpolated source controls and authored LUT image.
//! 00460530 -> 0045f230 / 0045ece0 / 0045e620, PS5b71154 and PS900b8491.
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::FullscreenShader,
    platform::collections::HashMap,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};

/// The 23 native vc_* vectors, before renderer code-constant normalization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NativeFilmControls {
    pub input_black: Vec4,
    pub input_white: Vec4,
    pub input_gamma: Vec4,
    pub output_black: Vec4,
    pub output_white: Vec4,
    pub rgb_high: Vec4,
    pub rgb_low: Vec4,
    pub y_high: Vec4,
    pub y_low: Vec4,
    pub ramp_start: Vec4,
    pub ramp_end: Vec4,
    pub shadow_red: Vec4,
    pub shadow_green: Vec4,
    pub shadow_blue: Vec4,
    pub mid_red: Vec4,
    pub mid_green: Vec4,
    pub mid_blue: Vec4,
    pub high_red: Vec4,
    pub high_green: Vec4,
    pub high_blue: Vec4,
    pub film_gamma: Vec4,
    pub film_saturation: Vec4,
    pub film_blend: Vec4,
}

impl NativeFilmControls {
    /// Parse an actual recovered .vision. Other vision/postfilter fields have
    /// separate owners; all ordinary film/bloom vectors must be present.
    pub fn parse_source(source: &str) -> Result<Self, String> {
        const NAMES: [&str; 23] = [
            "vc_LIB", "vc_LIW", "vc_LIG", "vc_LOB", "vc_LOW", "vc_RGBH", "vc_RGBL", "vc_YH",
            "vc_YL", "vc_RS", "vc_RE", "vc_SMR", "vc_SMG", "vc_SMB", "vc_MMR", "vc_MMG", "vc_MMB",
            "vc_HMR", "vc_HMG", "vc_HMB", "vc_FGM", "vc_FSM", "vc_FBM",
        ];
        let mut fields = HashMap::with_capacity(NAMES.len());
        for line in source.lines() {
            let mut words = line.split_whitespace();
            let Some(name) = words.next() else { continue };
            if !NAMES.contains(&name) {
                continue;
            }
            let value = line
                .split_once('"')
                .and_then(|(_, rest)| rest.split_once('"'))
                .map(|(value, _)| value)
                .ok_or_else(|| format!("source vision {name} requires a quoted vector"))?;
            let values = value
                .split_whitespace()
                .map(str::parse::<f32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| format!("source vision {name} has an invalid float"))?;
            let values: [f32; 4] = values
                .try_into()
                .map_err(|_| format!("source vision {name} requires four components"))?;
            let vector = Vec4::from_array(values);
            if !vector.is_finite() {
                return Err(format!("source vision {name} must be finite"));
            }
            fields.insert(name, vector);
        }
        let field = |name| {
            fields
                .get(name)
                .copied()
                .ok_or_else(|| format!("source vision is missing {name}"))
        };
        Ok(Self {
            input_black: field("vc_LIB")?,
            input_white: field("vc_LIW")?,
            input_gamma: field("vc_LIG")?,
            output_black: field("vc_LOB")?,
            output_white: field("vc_LOW")?,
            rgb_high: field("vc_RGBH")?,
            rgb_low: field("vc_RGBL")?,
            y_high: field("vc_YH")?,
            y_low: field("vc_YL")?,
            ramp_start: field("vc_RS")?,
            ramp_end: field("vc_RE")?,
            shadow_red: field("vc_SMR")?,
            shadow_green: field("vc_SMG")?,
            shadow_blue: field("vc_SMB")?,
            mid_red: field("vc_MMR")?,
            mid_green: field("vc_MMG")?,
            mid_blue: field("vc_MMB")?,
            high_red: field("vc_HMR")?,
            high_green: field("vc_HMG")?,
            high_blue: field("vc_HMB")?,
            film_gamma: field("vc_FGM")?,
            film_saturation: field("vc_FSM")?,
            film_blend: field("vc_FBM")?,
        })
    }

    /// Weight comes from the existing vision owner: sine-eased ordinary mode3,
    /// or the explicit linear fraction for native visionsetnakedlerp mode6.
    pub fn interpolate(self, target: Self, weight: f32) -> Self {
        assert!(weight.is_finite() && (0.0..=1.0).contains(&weight));
        let mix = |a: Vec4, b: Vec4| a + (b - a) * weight;
        Self {
            input_black: mix(self.input_black, target.input_black),
            input_white: mix(self.input_white, target.input_white),
            input_gamma: mix(self.input_gamma, target.input_gamma),
            output_black: mix(self.output_black, target.output_black),
            output_white: mix(self.output_white, target.output_white),
            rgb_high: mix(self.rgb_high, target.rgb_high),
            rgb_low: mix(self.rgb_low, target.rgb_low),
            y_high: mix(self.y_high, target.y_high),
            y_low: mix(self.y_low, target.y_low),
            ramp_start: mix(self.ramp_start, target.ramp_start),
            ramp_end: mix(self.ramp_end, target.ramp_end),
            shadow_red: mix(self.shadow_red, target.shadow_red),
            shadow_green: mix(self.shadow_green, target.shadow_green),
            shadow_blue: mix(self.shadow_blue, target.shadow_blue),
            mid_red: mix(self.mid_red, target.mid_red),
            mid_green: mix(self.mid_green, target.mid_green),
            mid_blue: mix(self.mid_blue, target.mid_blue),
            high_red: mix(self.high_red, target.high_red),
            high_green: mix(self.high_green, target.high_green),
            high_blue: mix(self.high_blue, target.high_blue),
            film_gamma: mix(self.film_gamma, target.film_gamma),
            film_saturation: mix(self.film_saturation, target.film_saturation),
            film_blend: mix(self.film_blend, target.film_blend),
        }
    }

    fn bloom_constants(self, extent: UVec2) -> BloomConstants {
        // 0045f230 preserves source RGB and Y thresholds, even a zero Y range.
        let input_white = Vec4::from_array(std::array::from_fn(|i| {
            if self.input_white[i] <= self.input_black[i] {
                self.input_black[i] + 0.00006103515625
            } else {
                self.input_white[i]
            }
        }));
        let output_white = Vec4::from_array(std::array::from_fn(|i| {
            if self.output_white[i] <= self.output_black[i] {
                self.output_black[i] + 0.00006103515625
            } else {
                self.output_white[i]
            }
        }));
        let input_scale = Vec4::ONE / (input_white - self.input_black);
        BloomConstants {
            thresholds: Vec4::new(
                self.rgb_high.w - self.rgb_low.w * self.rgb_high.w,
                self.y_high.w - self.y_low.w * self.y_high.w,
                self.rgb_high.w,
                self.y_high.w,
            ),
            input_scale,
            input_bias: -(input_scale * self.input_black),
            gamma: self.input_gamma,
            output_scale: output_white - self.output_black,
            output_bias: self.output_black,
            rgb_high: self.rgb_high,
            rgb_low: self.rgb_low,
            y_high: self.y_high,
            y_low: self.y_low,
            step: Vec4::new(1.0 / extent.x as f32, 1.0 / extent.y as f32, 0.0, 0.0),
        }
    }
}

/// LUT bank0 is the ordinary map bank. An actual source override may supply
/// the previous/current bank and blend; these are raw atlas samples, not two
/// already-graded endpoint LUTs. Color-control matrix is supplied by its owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NativeFilmSettings {
    pub controls: NativeFilmControls,
    pub lut_banks: [u32; 2],
    pub lut_weight: f32,
    pub color_matrix: [Vec4; 3],
}
impl NativeFilmSettings {
    pub fn ordinary(controls: NativeFilmControls) -> Self {
        Self {
            controls,
            lut_banks: [0, 0],
            lut_weight: 1.0,
            color_matrix: [Vec4::X, Vec4::Y, Vec4::Z],
        }
    }
    fn lut_constants(self) -> FilmConstants {
        let c = self.controls;
        let mut start = c.ramp_start;
        let mut end = c.ramp_end;
        // 0045e620's asymmetric ordered bounds, including RS.w versus RE.z.
        if end.x <= start.x {
            end.x = start.x + 0.000244140625;
        }
        if end.y <= start.y {
            start.y = end.y - 0.000244140625;
        }
        if end.z <= start.z {
            start.z = end.z - 0.000244140625;
        }
        if start.w <= end.z {
            start.w = end.z + 0.000244140625;
        }
        if end.w <= start.w {
            end.w = start.w + 0.000244140625;
        }
        let ramp_a = Vec4::ONE
            / Vec4::new(
                start.x - end.x,
                end.y - start.y,
                end.z - start.z,
                start.w - end.w,
            );
        let ramp_b = -(ramp_a * Vec4::new(end.x, start.y, start.z, end.w));
        let sum = c.film_saturation.x + c.film_saturation.y + c.film_saturation.z;
        FilmConstants {
            ramp_a,
            ramp_b,
            red_rows: [c.shadow_red, c.mid_red, c.high_red],
            green_rows: [c.shadow_green, c.mid_green, c.high_green],
            blue_rows: [c.shadow_blue, c.mid_blue, c.high_blue],
            gamma: c.film_gamma,
            saturation: (c.film_saturation.truncate() / sum).extend(c.film_saturation.w),
            blend: c.film_blend,
            banks: Vec4::new(
                self.lut_banks[0] as f32,
                self.lut_banks[1] as f32,
                self.lut_weight,
                1.0 - self.lut_weight,
            ),
            color_matrix: self.color_matrix,
        }
    }
}

#[derive(Clone, Copy, PartialEq, ShaderType)]
struct BloomConstants {
    thresholds: Vec4,
    input_scale: Vec4,
    input_bias: Vec4,
    gamma: Vec4,
    output_scale: Vec4,
    output_bias: Vec4,
    rgb_high: Vec4,
    rgb_low: Vec4,
    y_high: Vec4,
    y_low: Vec4,
    step: Vec4,
}
#[derive(Clone, Copy, PartialEq, ShaderType)]
struct FilmConstants {
    ramp_a: Vec4,
    ramp_b: Vec4,
    red_rows: [Vec4; 3],
    green_rows: [Vec4; 3],
    blue_rows: [Vec4; 3],
    gamma: Vec4,
    saturation: Vec4,
    blend: Vec4,
    banks: Vec4,
    color_matrix: [Vec4; 3],
}

/// Fixed source720-reference pyramid, independent of output pixel density.
pub(crate) fn bloom_extents(scene: UVec2) -> [UVec2; 3] {
    assert!(scene.x > 0 && scene.y > 0);
    let width = ((scene.x as f32 / scene.y as f32) * 720.0) as u32;
    [
        UVec2::new(width >> 2, 180),
        UVec2::new(width >> 3, 90),
        UVec2::new(width >> 4, 45),
    ]
}

pub(crate) struct NativeFilmPlugin;
impl Plugin for NativeFilmPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "native_film_bloom.wesl");
        embedded_asset!(app, "native_film_lut.wesl");
        embedded_asset!(app, "native_film_apply.wesl");
        let shaders = FilmShaders {
            bloom: load_embedded_asset!(app, "native_film_bloom.wesl"),
            lut: load_embedded_asset!(app, "native_film_lut.wesl"),
            apply: load_embedded_asset!(app, "native_film_apply.wesl"),
        };
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(shaders)
                .add_systems(RenderStartup, initialize_pipeline);
        }
    }
}
#[derive(Resource)]
struct FilmShaders {
    bloom: Handle<Shader>,
    lut: Handle<Shader>,
    apply: Handle<Shader>,
}
fn initialize_pipeline(
    mut commands: Commands,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    fullscreen: Res<FullscreenShader>,
    shaders: Res<FilmShaders>,
) {
    commands.insert_resource(NativeFilmPipeline::new(
        &device,
        &cache,
        &fullscreen,
        &shaders,
    ));
}

/// Pipeline/layout/sampler owner. Film output is RGBA8 encoded; UI and display
/// transfer are separate later consumers owned by the frame renderer.
#[derive(Resource)]
pub(crate) struct NativeFilmPipeline {
    single_layout: BindGroupLayout,
    combine_layout: BindGroupLayout,
    lut_layout: BindGroupLayout,
    apply_layout: BindGroupLayout,
    sampler: Sampler,
    pipelines: [CachedRenderPipelineId; 7],
}
impl NativeFilmPipeline {
    fn new(
        device: &RenderDevice,
        cache: &PipelineCache,
        fullscreen: &FullscreenShader,
        shaders: &FilmShaders,
    ) -> Self {
        let single = BindGroupLayoutDescriptor::new(
            "native bloom image and controls",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<BloomConstants>(false),
                ),
            ),
        );
        let mut combined = single.entries.to_vec();
        combined.push(
            texture_2d(TextureSampleType::Float { filterable: true })
                .build(3, ShaderStages::FRAGMENT),
        );
        let combine = BindGroupLayoutDescriptor::new("native bloom high and low", &combined);
        let lut = BindGroupLayoutDescriptor::new(
            "native authored LUT and film controls",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    uniform_buffer::<FilmConstants>(false),
                ),
            ),
        );
        let apply = BindGroupLayoutDescriptor::new(
            "native bloom/shoulder/LUT apply",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
            ),
        );
        let entries = [
            "downsample",
            "convolution",
            "blur_y",
            "blur_x",
            "combine",
            "generate",
            "apply",
        ];
        let pipelines = std::array::from_fn(|i| {
            let (layout, shader, format) = match i {
                4 => (&combine, &shaders.bloom, TextureFormat::Rgba16Float),
                5 => (&lut, &shaders.lut, TextureFormat::Rgb10a2Unorm),
                6 => (&apply, &shaders.apply, TextureFormat::Rgba8Unorm),
                _ => (&single, &shaders.bloom, TextureFormat::Rgba16Float),
            };
            cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(format!("native film {}", entries[i]).into()),
                layout: vec![layout.clone()],
                vertex: fullscreen.to_vertex_state(),
                fragment: Some(FragmentState {
                    shader: shader.clone(),
                    entry_point: Some(entries[i].into()),
                    targets: vec![Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                    ..default()
                }),
                ..default()
            })
        });
        Self {
            single_layout: cache.get_bind_group_layout(&single),
            combine_layout: cache.get_bind_group_layout(&combine),
            lut_layout: cache.get_bind_group_layout(&lut),
            apply_layout: cache.get_bind_group_layout(&apply),
            pipelines,
            sampler: device.create_sampler(&SamplerDescriptor {
                label: Some("native code sampler0x62"),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                lod_min_clamp: 0.0,
                lod_max_clamp: 0.0,
                ..default()
            }),
        }
    }
    pub fn is_ready(&self, cache: &PipelineCache) -> bool {
        self.pipelines
            .iter()
            .all(|id| cache.get_render_pipeline(*id).is_some())
    }
}

struct BloomPass {
    controls: UniformBuffer<BloomConstants>,
    group: BindGroup,
}
/// One camera's live film resources. No readback, texture cloning or allocation
/// occurs during rendering. Targets are replaced only when their extents change.
#[derive(Component)]
pub(crate) struct NativeFilmView {
    scene_extent: UVec2,
    input: TextureViewId,
    authored: TextureViewId,
    settings: NativeFilmSettings,
    bloom: [TextureView; 5],
    lut: TextureView,
    passes: [BloomPass; 7],
    film: UniformBuffer<FilmConstants>,
    lut_group: BindGroup,
    apply_group: BindGroup,
    lut_dirty: bool,
}

impl NativeFilmView {
    pub fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        pipeline: &NativeFilmPipeline,
        input: &TextureView,
        scene_extent: UVec2,
        authored: &GpuImage,
        settings: NativeFilmSettings,
    ) -> Result<Self, String> {
        validate_inputs(scene_extent, authored, settings)?;
        let sizes = bloom_extents(scene_extent);
        let bloom = std::array::from_fn(|i| {
            let size = sizes[match i {
                0 => 0,
                1 | 2 => 1,
                _ => 2,
            }];
            texture(
                device,
                size,
                TextureFormat::Rgba16Float,
                "native bloom pyramid",
            )
        });
        let lut = texture(
            device,
            UVec2::new(1024, 32),
            TextureFormat::Rgb10a2Unorm,
            "native generated film LUT",
        );
        let pass_extents = [
            scene_extent,
            sizes[0],
            sizes[2],
            sizes[2],
            sizes[1],
            sizes[1],
            sizes[1],
        ];
        let sources = [
            input, &bloom[0], &bloom[1], &bloom[4], &bloom[1], &bloom[2], &bloom[1],
        ];
        let passes = std::array::from_fn(|i| {
            let mut controls =
                UniformBuffer::from(settings.controls.bloom_constants(pass_extents[i]));
            controls.write_buffer(device, queue);
            let group = bloom_group(
                device,
                pipeline,
                sources[i],
                &controls,
                (i == 4).then_some(&bloom[3]),
            );
            BloomPass { controls, group }
        });
        let mut film = UniformBuffer::from(settings.lut_constants());
        film.write_buffer(device, queue);
        let lut_group = lut_group(device, pipeline, authored, &film);
        let apply_group = apply_group(device, pipeline, input, &bloom[2], &lut);
        Ok(Self {
            scene_extent,
            input: input.id(),
            authored: authored.texture_view.id(),
            settings,
            bloom,
            lut,
            passes,
            film,
            lut_group,
            apply_group,
            lut_dirty: true,
        })
    }

    pub fn update(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        pipeline: &NativeFilmPipeline,
        input: &TextureView,
        scene_extent: UVec2,
        authored: &GpuImage,
        settings: NativeFilmSettings,
    ) -> Result<(), String> {
        if settings != self.settings
            || scene_extent != self.scene_extent
            || authored.texture_view.id() != self.authored
        {
            validate_inputs(scene_extent, authored, settings)?;
        }
        if bloom_extents(scene_extent) != bloom_extents(self.scene_extent) {
            *self = Self::new(
                device,
                queue,
                pipeline,
                input,
                scene_extent,
                authored,
                settings,
            )?;
            return Ok(());
        }
        if settings.controls != self.settings.controls || scene_extent != self.scene_extent {
            let sizes = bloom_extents(scene_extent);
            let extents = [
                scene_extent,
                sizes[0],
                sizes[2],
                sizes[2],
                sizes[1],
                sizes[1],
                sizes[1],
            ];
            for (pass, extent) in self.passes.iter_mut().zip(extents) {
                let controls = settings.controls.bloom_constants(extent);
                if *pass.controls.get() != controls {
                    pass.controls.set(controls);
                    pass.controls.write_buffer(device, queue);
                }
            }
        }
        if settings != self.settings {
            let film = settings.lut_constants();
            if *self.film.get() != film {
                self.film.set(film);
                self.film.write_buffer(device, queue);
                self.lut_dirty = true;
            }
        }
        if authored.texture_view.id() != self.authored {
            self.lut_group = lut_group(device, pipeline, authored, &self.film);
            self.lut_dirty = true;
        }
        if input.id() != self.input {
            self.passes[0].group =
                bloom_group(device, pipeline, input, &self.passes[0].controls, None);
            self.apply_group = apply_group(device, pipeline, input, &self.bloom[2], &self.lut);
        }
        self.scene_extent = scene_extent;
        self.input = input.id();
        self.authored = authored.texture_view.id();
        self.settings = settings;
        Ok(())
    }

    /// Caller obtains the RGBA8 destination only after is_ready, before UI.
    /// False means no pass was recorded and the caller must retain its source.
    pub fn render(
        &mut self,
        context: &mut RenderContext,
        pipeline: &NativeFilmPipeline,
        cache: &PipelineCache,
        destination: &TextureView,
    ) -> bool {
        let [
            Some(down),
            Some(convolution),
            Some(blur_y),
            Some(blur_x),
            Some(combine),
            Some(generate),
            Some(apply),
        ] = pipeline.pipelines.map(|id| cache.get_render_pipeline(id))
        else {
            return false;
        };
        let programs = [down, convolution, blur_y, blur_x, combine, generate, apply];
        if self.lut_dirty {
            draw(
                context,
                &self.lut,
                programs[5],
                &self.lut_group,
                "native film LUT generation",
            );
            self.lut_dirty = false;
        }
        // Omit 00460c06: its mip4 downsample is overwritten before any read.
        let targets = [0, 1, 4, 3, 2, 1, 2];
        let programs_by_pass = [0, 1, 2, 3, 4, 2, 3];
        for i in 0..7 {
            draw(
                context,
                &self.bloom[targets[i]],
                programs[programs_by_pass[i]],
                &self.passes[i].group,
                "native source bloom",
            );
        }
        draw(
            context,
            destination,
            programs[6],
            &self.apply_group,
            "native bloom shoulder and film",
        );
        true
    }
}

fn validate_inputs(
    scene: UVec2,
    authored: &GpuImage,
    settings: NativeFilmSettings,
) -> Result<(), String> {
    if scene.x == 0 || scene.y == 0 || bloom_extents(scene)[2].x == 0 {
        return Err("native film requires a nonempty source view and bloom extent".into());
    }
    let d = &authored.texture_descriptor;
    if d.size
        != (Extent3d {
            width: 1024,
            height: 64,
            depth_or_array_layers: 1,
        })
        || d.format != TextureFormat::Rgba8Unorm
        || d.dimension != TextureDimension::D2
        || d.sample_count != 1
        || d.mip_level_count != 1
    {
        return Err(
            "native film requires the actual 1024x64, single-mip RGBA8_UNORM source LUT".into(),
        );
    }
    if settings.lut_banks.iter().any(|bank| *bank >= 2)
        || !settings.lut_weight.is_finite()
        || !(0.0..=1.0).contains(&settings.lut_weight)
    {
        return Err("native film LUT selection is outside its two source banks".into());
    }
    let c = settings.lut_constants();
    let source = settings.controls;
    let fields = [
        source.input_black,
        source.input_white,
        source.input_gamma,
        source.output_black,
        source.output_white,
        source.rgb_high,
        source.rgb_low,
        source.y_high,
        source.y_low,
        source.ramp_start,
        source.ramp_end,
        source.shadow_red,
        source.shadow_green,
        source.shadow_blue,
        source.mid_red,
        source.mid_green,
        source.mid_blue,
        source.high_red,
        source.high_green,
        source.high_blue,
        source.film_gamma,
        source.film_saturation,
        source.film_blend,
    ];
    let bloom = source.bloom_constants(scene);
    if !c.ramp_a.is_finite()
        || !c.ramp_b.is_finite()
        || !c.saturation.is_finite()
        || fields.iter().any(|field| !field.is_finite())
        || !bloom.input_scale.is_finite()
        || !bloom.output_scale.is_finite()
        || settings.color_matrix.iter().any(|row| !row.is_finite())
    {
        return Err("native film controls produce invalid required code constants".into());
    }
    Ok(())
}
fn texture(
    device: &RenderDevice,
    size: UVec2,
    format: TextureFormat,
    label: &'static str,
) -> TextureView {
    device
        .create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor::default())
}
fn bloom_group(
    device: &RenderDevice,
    pipeline: &NativeFilmPipeline,
    source: &TextureView,
    controls: &UniformBuffer<BloomConstants>,
    low: Option<&TextureView>,
) -> BindGroup {
    let mut entries =
        BindGroupEntries::sequential((source, &pipeline.sampler, controls.binding().unwrap()))
            .to_vec();
    if let Some(low) = low {
        entries.push(BindGroupEntry {
            binding: 3,
            resource: BindingResource::TextureView(low),
        });
    }
    device.create_bind_group(
        "native bloom inputs",
        if low.is_some() {
            &pipeline.combine_layout
        } else {
            &pipeline.single_layout
        },
        &entries,
    )
}
fn lut_group(
    device: &RenderDevice,
    pipeline: &NativeFilmPipeline,
    authored: &GpuImage,
    film: &UniformBuffer<FilmConstants>,
) -> BindGroup {
    device.create_bind_group(
        "native authored film input",
        &pipeline.lut_layout,
        &BindGroupEntries::sequential((&authored.texture_view, film.binding().unwrap())),
    )
}
fn apply_group(
    device: &RenderDevice,
    pipeline: &NativeFilmPipeline,
    input: &TextureView,
    bloom: &TextureView,
    lut: &TextureView,
) -> BindGroup {
    device.create_bind_group(
        "native final film inputs",
        &pipeline.apply_layout,
        &BindGroupEntries::sequential((input, &pipeline.sampler, bloom, lut)),
    )
}
fn draw(
    context: &mut RenderContext,
    destination: &TextureView,
    pipeline: &RenderPipeline,
    inputs: &BindGroup,
    label: &'static str,
) {
    let attachments = [Some(RenderPassColorAttachment {
        view: destination,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(Default::default()),
            store: StoreOp::Store,
        },
    })];
    let mut pass = context
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, inputs, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recovered zm_tomb.vision, SHA256fdf6f1c4e7e01bb1d1243ed018568d8749b1b44e197b7ca933affb9fd1007b81.
    const CLEAR: &str = r#"
vc_LIB "0 0 0 0"
vc_LIW "32 32 32 32"
vc_LIG "1 1 1 1"
vc_LOB "0 0 0 0"
vc_LOW "32 32 32 32"
vc_RGBH "0.105000 0.105000 0.105000 0.320000"
vc_RGBL "0.035000 0.035000 0.035000 1.000000"
vc_YH "0.060478 0.100240 0.135000 0.200000"
vc_YL "0.020159 0.033413 0.045000 1.000000"
vc_RS "0.000000 0.265000 0.000000 0.530000"
vc_RE "0.725000 1.000000 0.470000 1.000000"
vc_SMR "1 0 0 0"
vc_SMG "0 1 0 0"
vc_SMB "0 0 1 0"
vc_HMR "1 0 0 0"
vc_HMG "0 1 0 0"
vc_HMB "0 0 1 0"
vc_MMR "1 0 0 0"
vc_MMG "0 1 0 0"
vc_MMB "0 0 1 0"
vc_FGM "1 1 1 0"
vc_FSM "0.212585 0.715195 0.072220 0.908203"
vc_FBM "1 1 1 0"
"#;

    #[test]
    fn ordinary_bloom_extents_follow_source_reference_instead_of_output_density() {
        let expected = [
            UVec2::new(320, 180),
            UVec2::new(160, 90),
            UVec2::new(80, 45),
        ];
        assert_eq!(bloom_extents(UVec2::new(1280, 720)), expected);
        assert_eq!(bloom_extents(UVec2::new(3840, 2160)), expected);
        assert_eq!(
            bloom_extents(UVec2::new(1600, 1200)),
            [
                UVec2::new(240, 180),
                UVec2::new(120, 90),
                UVec2::new(60, 45)
            ]
        );
    }

    #[test]
    fn source_vision_parser_requires_authored_finite_film_inputs() {
        let clear = NativeFilmControls::parse_source(CLEAR).unwrap();
        assert_eq!(clear.rgb_high, Vec4::new(0.105, 0.105, 0.105, 0.32));
        assert_eq!(clear.film_saturation.w, 0.908203);
        assert!(
            NativeFilmControls::parse_source(&CLEAR.replace("vc_FGM", "missing_field")).is_err()
        );
        assert!(NativeFilmControls::parse_source(&CLEAR.replace("1 1 1 0", "NaN 1 1 0")).is_err());
        assert!(
            NativeFilmControls::parse_source(&CLEAR.replace("32 32 32 32", "32 32 32")).is_err()
        );
    }

    #[test]
    fn source_code_constants_keep_native_bounds_and_zero_y_threshold() {
        let clear = NativeFilmControls::parse_source(CLEAR).unwrap();
        let bloom = clear.bloom_constants(UVec2::new(1280, 720));
        assert_eq!(bloom.thresholds, Vec4::new(0., 0., 0.32, 0.2));
        assert_eq!(bloom.input_scale, Vec4::splat(1. / 32.));
        assert_eq!(bloom.output_scale, Vec4::splat(32.));
        let mut degenerate = clear;
        degenerate.ramp_start.w = 0.25; // Original compares RS.w against RE.z.
        degenerate.input_white = clear.input_black;
        degenerate.output_white = clear.output_black;
        degenerate.y_high = Vec4::ZERO; // Actual Zombie Blood/default YH.
        degenerate.y_low = Vec4::new(0., 0., 0., 0.9);
        let constants = NativeFilmSettings::ordinary(degenerate).lut_constants();
        let corrected = degenerate.ramp_end.z + 0.000244140625;
        assert_eq!(constants.ramp_a.w, 1. / (corrected - degenerate.ramp_end.w));
        assert_eq!(
            degenerate.bloom_constants(UVec2::ONE).input_scale,
            Vec4::splat(16384.)
        );
        assert_eq!(
            degenerate.bloom_constants(UVec2::ONE).output_scale,
            Vec4::splat(1. / 16384.)
        );
        assert_eq!(degenerate.bloom_constants(UVec2::ONE).thresholds.w, 0.);
        assert!(constants.ramp_a.is_finite() && constants.ramp_b.is_finite());
    }

    // Independent scalar reference for known finite source states. This checks
    // that the CPU code constants preserve the shader's authored tonal order.
    fn reference_grade(raw: Vec3, settings: NativeFilmSettings) -> Vec3 {
        let c = settings.lut_constants();
        let y = raw.dot(Vec3::new(
            0.21258544921875,
            0.7151947021484375,
            0.0722198486328125,
        ));
        let ramp = (Vec4::splat(y) * c.ramp_a + c.ramp_b).clamp(Vec4::ZERO, Vec4::ONE);
        let r = Vec3::new(ramp.x, ramp.z * ramp.w, ramp.y);
        let eased = r * r * (Vec3::splat(3.) - r * 2.);
        let weights = eased / eased.element_sum();
        let linear = (raw * raw).extend(1.);
        let channel = |rows: [Vec4; 3]| {
            let value = Vec3::new(
                linear.dot(rows[0]),
                linear.dot(rows[1]),
                linear.dot(rows[2]),
            )
            .dot(weights);
            value.clamp(0., 1.)
        };
        let graded = Vec3::new(
            channel(c.red_rows),
            channel(c.green_rows),
            channel(c.blue_rows),
        );
        let powered = Vec3::new(
            graded.x.powf(c.gamma.x),
            graded.y.powf(c.gamma.y),
            graded.z.powf(c.gamma.z),
        );
        let grey = powered.dot(c.saturation.truncate());
        let saturated = Vec3::splat(grey) + (powered - Vec3::splat(grey)) * c.saturation.w;
        let mixed =
            (linear.truncate() + c.blend.truncate() * (saturated - linear.truncate())).extend(1.);
        let result = Vec3::new(
            mixed.dot(c.color_matrix[0]),
            mixed.dot(c.color_matrix[1]),
            mixed.dot(c.color_matrix[2]),
        )
        .clamp(Vec3::ZERO, Vec3::ONE);
        Vec3::new(result.x.sqrt(), result.y.sqrt(), result.z.sqrt())
    }

    #[test]
    fn weather_lut_grades_current_parameters_before_lattice_quantization() {
        let clear = NativeFilmControls::parse_source(CLEAR).unwrap();
        let mut rain = clear;
        // Actual zm_tomb_rain / zm_tomb_snow source controls.
        rain.film_gamma = Vec4::new(0.887097, 0.887097, 0.887097, 0.);
        rain.film_blend = Vec4::ZERO;
        let raw = Vec3::new(0.8, 0.3, 0.1);
        let a = reference_grade(raw, NativeFilmSettings::ordinary(clear));
        let b = reference_grade(raw, NativeFilmSettings::ordinary(rain));
        let grey = (raw * raw).dot(clear.film_saturation.truncate());
        let expected = Vec3::splat(grey) + ((raw * raw) - Vec3::splat(grey)) * 0.908203;
        assert!((a * a - expected).abs().max_element() < 0.000001);
        assert!((b - raw).abs().max_element() < 0.000001);
        let mixed = reference_grade(
            raw,
            NativeFilmSettings::ordinary(clear.interpolate(rain, 0.5)),
        );
        assert!((mixed - (a + b) * 0.5).abs().max_element() > 0.001);
        for sample in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE, raw] {
            let graded = reference_grade(sample, NativeFilmSettings::ordinary(clear));
            assert!(graded.is_finite());
            // Source RGB10A2 quantizes generated lattice entries to ten bits.
            let quantized = (graded * 1023.).round() / 1023.;
            assert!((quantized - graded).abs().max_element() <= 0.5 / 1023. + f32::EPSILON);
        }
    }
}
