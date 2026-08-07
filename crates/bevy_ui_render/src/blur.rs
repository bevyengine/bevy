//! Renders blurred regions of the screen behind UI nodes.
//!
//! Add a [`BlurRegionCamera`] to a camera and tag UI nodes with [`BlurRegion`].
//! Every frame the plugin mirrors the layout rectangles of tagged nodes into the
//! camera, and the selected [`BlurSetting`] algorithm is applied to those regions
//! as a post process.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::{
    tonemapping::tonemapping, Core2d, Core2dSystems, Core3d, Core3dSystems, FullscreenShader,
};
use bevy_ecs::{
    component::Component,
    prelude::{Commands, Entity, Query, Res, ResMut, With},
    query::QueryItem,
    resource::Resource,
    schedule::IntoScheduleConfigs,
};
use bevy_math::{Rect, Vec4};
use bevy_render::{
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        AddressMode, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d,
        FilterMode, FragmentState, MultisampleState, Operations, PipelineCache, PrimitiveState,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
        Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType,
        SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages, TextureView,
    },
    renderer::{RenderContext, RenderDevice, ViewQuery},
    sync_component::SyncComponent,
    texture::{CachedTexture, TextureCache},
    view::ViewTarget,
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_ui::{ComputedNode, ComputedUiTargetCamera, UiGlobalTransform, UiSystems};
use bevy_utils::default;
use tracing::warn;

/// The texture format used for intermediate blur render targets.
const INTERMEDIATE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

/// The default maximum number of blur regions per camera. This is a compile-time constant to allow the shader array sizes to be known at compile time;
/// if you need more regions, create a custom [`BlurRegionCamera`] with a larger `N` and register a matching `BlurShaderPlugin::<N>`.
pub const DEFAULT_MAX_BLUR_REGIONS_COUNT: usize = 32;

/// Parameters of the single-component complex kernel used by [`BlurSetting::Bokeh`].
/// The constants are drawn from <http://yehar.com/blog/?p=1495> via
/// <https://github.com/mikepound/convolve/blob/master/complex_kernels.py> (1-component row).
///
/// These must stay in sync with the constants of the same names in `blur.wgsl`.
const BOKEH_KERNEL_A: f32 = 0.862325;
const BOKEH_KERNEL_B: f32 = 1.624835;
const BOKEH_WEIGHT_REAL: f32 = 0.767583;
const BOKEH_WEIGHT_IMAG: f32 = 1.862321;
const BOKEH_KERNEL_SCALE: f32 = 1.4;

/// Selects the blur algorithm and its parameters for a [`BlurRegionCamera`].
///
/// All algorithms can have their parameters changed freely at runtime. Different algorithms can provide different tradeoffs
/// between cost and quality. The best choice depends on the blur size, the content being blurred, and personal taste.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlurSetting {
    /// A separable box blur: every pixel in the kernel contributes equally.
    ///
    /// A somewhat expensive and visually the crudest, producing visible
    /// streaks for large radii. Runs as a horizontal and a vertical pass.
    BoxBlur {
        /// Number of pixels sampled on each side of the center, per axis.
        /// The kernel covers `2 * kernel_radius + 1` pixels in each direction.
        kernel_radius: u32,
        /// Multiplier for the spacing between samples, in pixels.
        /// `1.0` samples adjacent pixels; larger values widen the blur for free
        /// at the cost of slight grain (bilinear filtering hides most of it).
        scale: f32,
    },
    /// A separable Gaussian blur.
    ///
    /// A expensive but way more beautiful blur algorithm. Uses bilinear filtering to halve the number of texture samples.
    /// Runs as a horizontal and a vertical pass.
    Gaussian {
        /// The diameter, in physical pixels, of the circle of confusion: the area
        /// around each pixel that contributes to the blur. Larger is blurrier.
        /// <https://en.wikipedia.org/wiki/Circle_of_confusion>
        circle_of_confusion: f32,
        /// Standard deviation (σ) of the kernel as a fraction of the circle of
        /// confusion. The default of `0.25` means σ = `CoC` × 0.25, which keeps
        /// virtually all of the kernel's weight inside the circle of confusion.
        sigma_multiplier: f32,
    },
    /// A dual Kawase blur: downsamples the scene through a chain of progressively
    /// half-resolution textures and upsamples back, blurring at each step.
    ///
    /// Produces very smooth, wide blurs at a fraction of the cost of an equally
    /// wide Gaussian, since most passes run at reduced resolution. The strength
    /// grows roughly exponentially with `mip_count`.
    DualKawase {
        /// Number of downsample levels, `1..=6`. Level `n` runs at `1/2ⁿ` resolution.
        mip_count: u32,
        /// Sample offset multiplier used by every pass. Typical range `0.5..=4.0`;
        /// larger values blur more aggressively but can shimmer beyond ~3.0.
        offset: f32,
    },
    /// A bokeh blur that mimics a camera aperture: out-of-focus highlights bloom
    /// into bright, hard-edged discs instead of smearing out.
    ///
    /// Implemented as a separable convolution with a single-component complex
    /// kernel whose magnitude approximates a disc (see `BOKEH_KERNEL_A`).
    /// Runs as a horizontal pass into two intermediate textures holding the
    /// complex response, then a vertical pass that resolves them to a color.
    /// The most expensive algorithm: cost scales linearly with `radius`.
    Bokeh {
        /// Radius of the aperture disc in physical pixels, `1..=64`.
        radius: u32,
    },
}

impl BlurSetting {
    /// Reasonable defaults for each algorithm, handy as starting points.
    pub const BOX_BLUR: BlurSetting = BlurSetting::BoxBlur {
        kernel_radius: 8,
        scale: 2.0,
    };
    /// See [`BlurSetting::BOX_BLUR`].
    pub const GAUSSIAN: BlurSetting = BlurSetting::Gaussian {
        circle_of_confusion: 100.0,
        sigma_multiplier: 0.25,
    };
    /// See [`BlurSetting::BOX_BLUR`].
    pub const DUAL_KAWASE: BlurSetting = BlurSetting::DualKawase {
        mip_count: 3,
        offset: 1.5,
    };
    /// See [`BlurSetting::BOX_BLUR`].
    pub const BOKEH: BlurSetting = BlurSetting::Bokeh { radius: 24 };

    /// Packs the algorithm parameters into the generic `params` uniform vector.
    /// The meaning of each component is algorithm specific; see `blur.wgsl`.
    fn shader_params(&self) -> Vec4 {
        match *self {
            BlurSetting::BoxBlur {
                kernel_radius,
                scale,
            } => Vec4::new(kernel_radius as f32, scale.max(0.0), 0.0, 0.0),
            BlurSetting::Gaussian {
                circle_of_confusion,
                sigma_multiplier,
            } => Vec4::new(
                circle_of_confusion.max(0.0),
                sigma_multiplier.max(0.001),
                0.0,
                0.0,
            ),
            BlurSetting::DualKawase { offset, .. } => Vec4::new(offset.max(0.0), 0.0, 0.0, 0.0),
            BlurSetting::Bokeh { radius } => {
                let radius = radius.clamp(1, 64);
                Vec4::new(radius as f32, 1.0 / bokeh_normalization(radius), 0.0, 0.0)
            }
        }
    }
}

impl Default for BlurSetting {
    fn default() -> Self {
        BlurSetting::GAUSSIAN
    }
}

/// Computes the normalization factor of the 2D bokeh kernel for a given radius.
///
/// Following `normalize_kernels` in the reference implementation, the 2D kernel is
/// the outer product of the 1D complex kernel with itself, and the normalization is
/// the weighted sum `Σᵢⱼ A·Re(kᵢ·kⱼ) + B·Im(kᵢ·kⱼ)`. Because `Σᵢⱼ kᵢ·kⱼ = (Σᵢ kᵢ)²`,
/// this reduces to evaluating the square of the 1D kernel sum.
fn bokeh_normalization(radius: u32) -> f32 {
    let r = radius.max(1) as i32;
    let mut sum_re = 0.0f32;
    let mut sum_im = 0.0f32;
    for x in -r..=r {
        let t = x as f32 * BOKEH_KERNEL_SCALE / r as f32;
        let t2 = t * t;
        let e = bevy_math::ops::exp(-BOKEH_KERNEL_A * t2);
        sum_re += e * bevy_math::ops::cos(BOKEH_KERNEL_B * t2);
        sum_im += e * bevy_math::ops::sin(BOKEH_KERNEL_B * t2);
    }
    let total = BOKEH_WEIGHT_REAL * (sum_re * sum_re - sum_im * sum_im)
        + BOKEH_WEIGHT_IMAG * (2.0 * sum_re * sum_im);
    // The kernel sum is strictly positive for the parameter set above, but guard
    // against division by zero so a bad parameter tweak degrades instead of NaNs.
    total.max(f32::EPSILON)
}

/// A plugin that adds support for rendering blurred regions behind UI nodes.
pub struct BlurShaderPlugin<const N: usize>;

impl<const N: usize> Plugin for BlurShaderPlugin<N> {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "blur.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<BlurRegionCamera<N>>::default(),
            UniformComponentPlugin::<BlurRegionUniform<N>>::default(),
        ))
        .add_systems(PostUpdate, sync_blur_regions::<N>.after(UiSystems::Layout));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<BlurRegionPipeline<N>>>()
            .add_systems(RenderStartup, init_blur_pipeline::<N>)
            .add_systems(
                Render,
                prepare_blur_regions_passes::<N>.in_set(RenderSystems::Prepare),
            )
            .add_systems(
                Core3d,
                blur_regions_pass::<N>
                    .after(tonemapping)
                    .in_set(Core3dSystems::PostProcess),
            )
            .add_systems(
                Core2d,
                blur_regions_pass::<N>
                    .after(tonemapping)
                    .in_set(Core2dSystems::PostProcess),
            );
    }
}

// Components and systems

/// Add this marker component to a UI node to create a blur region behind it.
///
/// The node's layout rectangle and border radius are mirrored into the
/// [`BlurRegionCamera`] of the camera that renders the node, every frame.
#[derive(Component, Default, Clone, Copy)]
pub struct BlurRegion;

/// The final computed values of a blur region, in physical pixels.
#[derive(Default, Debug, Clone, ShaderType)]
struct ComputedBlurRegion {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    border_radii: Vec4,
}

impl ComputedBlurRegion {
    const OFFSCREEN: ComputedBlurRegion = ComputedBlurRegion {
        min_x: -1.0,
        max_x: -1.0,
        min_y: -1.0,
        max_y: -1.0,
        border_radii: Vec4::ZERO,
    };
}

/// Indicates that this camera should render blur regions, and selects the blur
/// algorithm via [`BlurSetting`].
///
/// Regions are normally populated automatically from UI nodes tagged with
/// [`BlurRegion`]. Regions can also be pushed manually with [`Self::blur`] and
/// friends, from a system scheduled after `sync_blur_regions` in [`PostUpdate`]
/// (the sync system rebuilds the region list each frame).
#[derive(Component, Debug, Clone)]
pub struct BlurRegionCamera<const N: usize> {
    /// The blur algorithm and its parameters. Can be changed freely at runtime.
    pub settings: BlurSetting,
    current_regions_count: u32,
    regions: [ComputedBlurRegion; N],
}

impl Default for BlurRegionCamera<DEFAULT_MAX_BLUR_REGIONS_COUNT> {
    fn default() -> Self {
        Self::new(BlurSetting::default())
    }
}

impl BlurRegionCamera<DEFAULT_MAX_BLUR_REGIONS_COUNT> {
    /// Creates a camera with the default maximum number of blur regions.
    pub fn new(settings: BlurSetting) -> Self {
        Self::with_settings(settings)
    }
}

impl<const N: usize> BlurRegionCamera<N> {
    /// Creates a camera with a custom maximum number of blur regions.
    /// Requires registering a matching `BlurShaderPlugin::<N>`.
    pub fn with_settings(settings: BlurSetting) -> Self {
        BlurRegionCamera {
            settings,
            current_regions_count: 0,
            regions: core::array::from_fn(|_| ComputedBlurRegion::OFFSCREEN),
        }
    }

    /// Adds a rectangular blur region, in physical pixels.
    pub fn blur(&mut self, rect: Rect) {
        self.rounded_blur(rect, Vec4::ZERO);
    }

    /// Adds a rounded rectangular blur region, in physical pixels.
    pub fn rounded_blur(&mut self, rect: Rect, border_radii: Vec4) {
        if self.current_regions_count == N as u32 {
            warn!("Blur region ignored as the max blur region count has already been reached");
            return;
        }

        self.regions[self.current_regions_count as usize] = ComputedBlurRegion {
            min_x: rect.min.x,
            max_x: rect.max.x,
            min_y: rect.min.y,
            max_y: rect.max.y,
            border_radii,
        };
        self.current_regions_count += 1;
    }

    pub fn blur_all(&mut self, rect: &[Rect]) {
        for rect in rect {
            self.blur(*rect);
        }
    }

    pub fn rounded_blur_all(&mut self, rect: &[(Rect, Vec4)]) {
        for rect in rect {
            self.rounded_blur(rect.0, rect.1);
        }
    }

    /// Removes all blur regions. Called automatically every frame by `sync_blur_regions`.
    pub fn clear(&mut self) {
        self.current_regions_count = 0;
    }
}

/// Mirrors the layout rectangles of UI nodes tagged with [`BlurRegion`] into the
/// [`BlurRegionCamera`] of the camera that renders them.
pub fn sync_blur_regions<const N: usize>(
    mut cameras: Query<&mut BlurRegionCamera<N>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform, &ComputedUiTargetCamera), With<BlurRegion>>,
) {
    for mut camera in &mut cameras {
        camera.clear();
    }

    for (node, transform, target) in &nodes {
        if node.is_empty() {
            continue;
        }
        let Some(camera_entity) = target.get() else {
            continue;
        };
        let Ok(mut camera) = cameras.get_mut(camera_entity) else {
            continue;
        };

        let rect = Rect::from_center_size(transform.translation, node.size());
        let border_radii = Vec4::from_array(node.border_radius.into());
        camera.rounded_blur(rect, border_radii);
    }
}

// Extraction

/// The render world copy of [`BlurRegionCamera::settings`], used to decide which
/// passes and intermediate textures each view needs.
#[derive(Component, Debug, Clone, Copy)]
pub struct ExtractedBlurSettings(pub BlurSetting);

/// The GPU uniform shared by every blur pass. `params` is interpreted per
/// algorithm; see the parameter documentation in `blur.wgsl`.
#[derive(Component, Clone, ShaderType)]
pub struct BlurRegionUniform<const N: usize> {
    params: Vec4,
    current_regions_count: u32,
    regions: [ComputedBlurRegion; N],
}

impl<const N: usize> SyncComponent for BlurRegionCamera<N> {
    type Target = (BlurRegionUniform<N>, ExtractedBlurSettings);
}

impl<const N: usize> ExtractComponent for BlurRegionCamera<N> {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = (BlurRegionUniform<N>, ExtractedBlurSettings);

    fn extract_component(camera: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some((
            BlurRegionUniform {
                params: camera.settings.shader_params(),
                current_regions_count: camera.current_regions_count,
                regions: camera.regions.clone(),
            },
            ExtractedBlurSettings(camera.settings),
        ))
    }
}

// Pipelines

#[derive(Resource)]
pub struct BlurRegionPipeline<const N: usize> {
    /// Layout for passes reading a single texture: gaussian, box, kawase
    /// down/upsample and the bokeh horizontal pass.
    single_input_layout: BindGroupLayout,
    single_input_layout_descriptor: BindGroupLayoutDescriptor,
    /// Layout for the kawase composite pass: the original scene plus the blurred chain.
    composite_layout: BindGroupLayout,
    composite_layout_descriptor: BindGroupLayoutDescriptor,
    /// Layout for the bokeh vertical pass: the original scene plus the two
    /// complex-response textures produced by the horizontal pass.
    bokeh_layout: BindGroupLayout,
    bokeh_layout_descriptor: BindGroupLayoutDescriptor,
    sampler: Sampler,
    fullscreen_shader: FullscreenShader,
    shader: Handle<Shader>,
}

fn init_blur_pipeline<const N: usize>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(BlurRegionPipeline::<N>::new(
        &render_device,
        fullscreen_shader.clone(),
        load_embedded_asset!(asset_server.as_ref(), "blur.wgsl"),
    ));
}

impl<const N: usize> BlurRegionPipeline<N> {
    fn new(
        render_device: &RenderDevice,
        fullscreen_shader: FullscreenShader,
        shader: Handle<Shader>,
    ) -> Self {
        let single_input_entries = BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<BlurRegionUniform<N>>(true),
            ),
        );
        let composite_entries = BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<BlurRegionUniform<N>>(true),
                texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        );
        let bokeh_entries = BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<BlurRegionUniform<N>>(true),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        );

        let single_input_layout = render_device
            .create_bind_group_layout("blur_single_input_layout", &single_input_entries);
        let single_input_layout_descriptor = BindGroupLayoutDescriptor::new(
            "blur_single_input_layout",
            single_input_entries.to_vec().leak(),
        );
        let composite_layout =
            render_device.create_bind_group_layout("blur_composite_layout", &composite_entries);
        let composite_layout_descriptor = BindGroupLayoutDescriptor::new(
            "blur_composite_layout",
            composite_entries.to_vec().leak(),
        );
        let bokeh_layout =
            render_device.create_bind_group_layout("blur_bokeh_layout", &bokeh_entries);
        let bokeh_layout_descriptor =
            BindGroupLayoutDescriptor::new("blur_bokeh_layout", bokeh_entries.to_vec().leak());

        // Linear filtering is load-bearing: the gaussian pass samples between texel
        // centers to fetch two texels at once, and the kawase and bokeh passes rely
        // on bilinear interpolation when sampling across resolutions.
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::MirrorRepeat,
            address_mode_v: AddressMode::MirrorRepeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        });

        Self {
            single_input_layout,
            single_input_layout_descriptor,
            composite_layout,
            composite_layout_descriptor,
            bokeh_layout,
            bokeh_layout_descriptor,
            sampler,
            fullscreen_shader,
            shader,
        }
    }
}

/// One fullscreen pass of a blur algorithm, keyed to a shader entry point.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum BlurPass {
    GaussianHorizontal,
    GaussianVertical,
    BoxHorizontal,
    BoxVertical,
    KawaseDownsample,
    KawaseUpsample,
    KawaseComposite,
    BokehHorizontal,
    BokehVertical,
}

impl BlurPass {
    fn entry_point(&self) -> &'static str {
        match self {
            BlurPass::GaussianHorizontal => "gaussian_horizontal",
            BlurPass::GaussianVertical => "gaussian_vertical",
            BlurPass::BoxHorizontal => "box_horizontal",
            BlurPass::BoxVertical => "box_vertical",
            BlurPass::KawaseDownsample => "kawase_downsample",
            BlurPass::KawaseUpsample => "kawase_upsample",
            BlurPass::KawaseComposite => "kawase_composite",
            BlurPass::BokehHorizontal => "bokeh_horizontal",
            BlurPass::BokehVertical => "bokeh_vertical",
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct BlurRegionPipelineKey {
    pass: BlurPass,
    /// The format of the view target; intermediate passes ignore this and render
    /// to [`INTERMEDIATE_TEXTURE_FORMAT`].
    target_format: TextureFormat,
}

impl<const N: usize> SpecializedRenderPipeline for BlurRegionPipeline<N> {
    type Key = BlurRegionPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let layout = match key.pass {
            BlurPass::KawaseComposite => self.composite_layout_descriptor.clone(),
            BlurPass::BokehVertical => self.bokeh_layout_descriptor.clone(),
            _ => self.single_input_layout_descriptor.clone(),
        };

        let intermediate_target = Some(ColorTargetState {
            format: INTERMEDIATE_TEXTURE_FORMAT,
            blend: None,
            write_mask: ColorWrites::ALL,
        });
        let targets = match key.pass {
            // The horizontal bokeh pass writes the complex response of all three
            // color channels across two textures.
            BlurPass::BokehHorizontal => vec![intermediate_target.clone(), intermediate_target],
            BlurPass::KawaseDownsample | BlurPass::KawaseUpsample => vec![intermediate_target],
            _ => vec![Some(ColorTargetState {
                format: key.target_format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        };

        RenderPipelineDescriptor {
            label: Some(format!("blur_pipeline_{}", key.pass.entry_point()).into()),
            layout: vec![layout],
            vertex: self.fullscreen_shader.to_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![ShaderDefVal::UInt(
                    "MAX_BLUR_REGIONS_COUNT".into(),
                    N as u32,
                )],
                entry_point: Some(key.pass.entry_point().into()),
                targets,
                constants: Vec::new(),
            }),
            ..default()
        }
    }
}

// Pass preparation

/// The prepared pipelines and intermediate textures for one view, shaped by the
/// view's [`BlurSetting`]. Only the resources the selected algorithm actually
/// needs are created.
#[derive(Component)]
pub enum BlurRegionPasses {
    /// Two ping-pong passes over the view target: gaussian and box blur.
    Separable {
        horizontal: CachedRenderPipelineId,
        vertical: CachedRenderPipelineId,
    },
    /// Dual kawase: downsample through `chain`, upsample back, then composite
    /// into the view target with region masking.
    DualKawase {
        downsample: CachedRenderPipelineId,
        upsample: CachedRenderPipelineId,
        composite: CachedRenderPipelineId,
        /// Progressively half-resolution textures; `chain[0]` is half the view size.
        chain: Vec<CachedTexture>,
    },
    /// Bokeh: a horizontal pass producing the complex kernel response, then a
    /// vertical pass resolving it back to color.
    Bokeh {
        horizontal: CachedRenderPipelineId,
        vertical: CachedRenderPipelineId,
        textures: Box<BokehTextures>,
    },
}

/// The intermediate textures holding the complex kernel response of the bokeh
/// horizontal pass.
pub struct BokehTextures {
    /// Complex response of the red and green channels: `(R.re, R.im, G.re, G.im)`.
    red_green: CachedTexture,
    /// Complex response of the blue channel: `(B.re, B.im, 0, 0)`.
    blue: CachedTexture,
}

fn prepare_blur_regions_passes<const N: usize>(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlurRegionPipeline<N>>>,
    pipeline: Res<BlurRegionPipeline<N>>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    views: Query<(
        Entity,
        &ViewTarget,
        &ExtractedBlurSettings,
        &BlurRegionUniform<N>,
    )>,
) {
    for (entity, view_target, settings, uniform) in &views {
        if uniform.current_regions_count == 0 {
            commands.entity(entity).remove::<BlurRegionPasses>();
            continue;
        }

        let target_format = view_target.main_texture_format();
        let mut specialize = |pass| {
            pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                BlurRegionPipelineKey {
                    pass,
                    target_format,
                },
            )
        };

        let passes = match settings.0 {
            BlurSetting::Gaussian { .. } => BlurRegionPasses::Separable {
                horizontal: specialize(BlurPass::GaussianHorizontal),
                vertical: specialize(BlurPass::GaussianVertical),
            },
            BlurSetting::BoxBlur { .. } => BlurRegionPasses::Separable {
                horizontal: specialize(BlurPass::BoxHorizontal),
                vertical: specialize(BlurPass::BoxVertical),
            },
            BlurSetting::DualKawase { mip_count, .. } => {
                let downsample = specialize(BlurPass::KawaseDownsample);
                let upsample = specialize(BlurPass::KawaseUpsample);
                let composite = specialize(BlurPass::KawaseComposite);

                let size = view_target.main_texture().size();
                let chain = (1..=mip_count.clamp(1, 6))
                    .map(|mip| {
                        texture_cache.get(
                            &render_device,
                            TextureDescriptor {
                                label: Some("blur_regions_dual_kawase_target"),
                                size: Extent3d {
                                    width: (size.width >> mip).max(1),
                                    height: (size.height >> mip).max(1),
                                    depth_or_array_layers: 1,
                                },
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: TextureDimension::D2,
                                format: INTERMEDIATE_TEXTURE_FORMAT,
                                usage: TextureUsages::RENDER_ATTACHMENT
                                    | TextureUsages::TEXTURE_BINDING,
                                view_formats: &[],
                            },
                        )
                    })
                    .collect();

                BlurRegionPasses::DualKawase {
                    downsample,
                    upsample,
                    composite,
                    chain,
                }
            }
            BlurSetting::Bokeh { .. } => {
                let horizontal = specialize(BlurPass::BokehHorizontal);
                let vertical = specialize(BlurPass::BokehVertical);

                let descriptor = |label: &'static str| TextureDescriptor {
                    label: Some(label),
                    size: view_target.main_texture().size(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: INTERMEDIATE_TEXTURE_FORMAT,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                };

                BlurRegionPasses::Bokeh {
                    horizontal,
                    vertical,
                    textures: Box::new(BokehTextures {
                        red_green: texture_cache
                            .get(&render_device, descriptor("blur_regions_bokeh_red_green")),
                        blue: texture_cache
                            .get(&render_device, descriptor("blur_regions_bokeh_blue")),
                    }),
                }
            }
        };

        commands.entity(entity).insert(passes);
    }
}

// Rendering
fn blur_regions_pass<const N: usize>(
    view: ViewQuery<(
        &ViewTarget,
        &BlurRegionPasses,
        &DynamicUniformIndex<BlurRegionUniform<N>>,
    )>,
    blur_regions_pipeline: Res<BlurRegionPipeline<N>>,
    pipeline_cache: Res<PipelineCache>,
    blur_regions_uniforms: Res<ComponentUniforms<BlurRegionUniform<N>>>,
    mut render_context: RenderContext,
) {
    let (view_target, passes, uniform_index) = view.into_inner();

    let Some(uniform_binding) = blur_regions_uniforms.uniforms().binding() else {
        return;
    };
    let uniform_offset = uniform_index.index();

    match passes {
        BlurRegionPasses::Separable {
            horizontal,
            vertical,
        } => {
            let (Some(horizontal), Some(vertical)) = (
                pipeline_cache.get_render_pipeline(*horizontal),
                pipeline_cache.get_render_pipeline(*vertical),
            ) else {
                return;
            };

            for (label, pipeline) in [
                ("blur_regions_horizontal_pass", horizontal),
                ("blur_regions_vertical_pass", vertical),
            ] {
                let post_process = view_target.post_process_write();
                let bind_group = render_context.render_device().create_bind_group(
                    "blur_regions_bind_group",
                    &blur_regions_pipeline.single_input_layout,
                    &BindGroupEntries::sequential((
                        post_process.source,
                        &blur_regions_pipeline.sampler,
                        uniform_binding.clone(),
                    )),
                );

                run_blur_pass(
                    &mut render_context,
                    label,
                    pipeline,
                    &bind_group,
                    uniform_offset,
                    &[color_attachment(post_process.destination)],
                );
            }
        }
        BlurRegionPasses::DualKawase {
            downsample,
            upsample,
            composite,
            chain,
        } => {
            let (Some(downsample), Some(upsample), Some(composite)) = (
                pipeline_cache.get_render_pipeline(*downsample),
                pipeline_cache.get_render_pipeline(*upsample),
                pipeline_cache.get_render_pipeline(*composite),
            ) else {
                return;
            };

            let post_process = view_target.post_process_write();

            let single_bind_group = |render_context: &RenderContext, source: &TextureView| {
                render_context.render_device().create_bind_group(
                    "blur_regions_bind_group",
                    &blur_regions_pipeline.single_input_layout,
                    &BindGroupEntries::sequential((
                        source,
                        &blur_regions_pipeline.sampler,
                        uniform_binding.clone(),
                    )),
                )
            };

            // Downsample the scene through the chain of half-resolution textures.
            let mut source = post_process.source;
            for texture in chain {
                let bind_group = single_bind_group(&render_context, source);
                run_blur_pass(
                    &mut render_context,
                    "blur_regions_kawase_downsample_pass",
                    downsample,
                    &bind_group,
                    uniform_offset,
                    &[color_attachment(&texture.default_view)],
                );
                source = &texture.default_view;
            }

            // Upsample back up the chain, stopping at the half-resolution level.
            for i in (1..chain.len()).rev() {
                let bind_group = single_bind_group(&render_context, &chain[i].default_view);
                run_blur_pass(
                    &mut render_context,
                    "blur_regions_kawase_upsample_pass",
                    upsample,
                    &bind_group,
                    uniform_offset,
                    &[color_attachment(&chain[i - 1].default_view)],
                );
            }

            // The final upsample to full resolution also selects, per pixel, between
            // the blurred result and the untouched scene based on the blur regions.
            let bind_group = render_context.render_device().create_bind_group(
                "blur_regions_composite_bind_group",
                &blur_regions_pipeline.composite_layout,
                &BindGroupEntries::sequential((
                    post_process.source,
                    &blur_regions_pipeline.sampler,
                    uniform_binding.clone(),
                    &chain[0].default_view,
                )),
            );
            run_blur_pass(
                &mut render_context,
                "blur_regions_kawase_composite_pass",
                composite,
                &bind_group,
                uniform_offset,
                &[color_attachment(post_process.destination)],
            );
        }
        BlurRegionPasses::Bokeh {
            horizontal,
            vertical,
            textures,
        } => {
            let BokehTextures { red_green, blue } = textures.as_ref();
            let (Some(horizontal), Some(vertical)) = (
                pipeline_cache.get_render_pipeline(*horizontal),
                pipeline_cache.get_render_pipeline(*vertical),
            ) else {
                return;
            };

            let post_process = view_target.post_process_write();

            let bind_group = render_context.render_device().create_bind_group(
                "blur_regions_bind_group",
                &blur_regions_pipeline.single_input_layout,
                &BindGroupEntries::sequential((
                    post_process.source,
                    &blur_regions_pipeline.sampler,
                    uniform_binding.clone(),
                )),
            );
            run_blur_pass(
                &mut render_context,
                "blur_regions_bokeh_horizontal_pass",
                horizontal,
                &bind_group,
                uniform_offset,
                &[
                    color_attachment(&red_green.default_view),
                    color_attachment(&blue.default_view),
                ],
            );

            let bind_group = render_context.render_device().create_bind_group(
                "blur_regions_bokeh_bind_group",
                &blur_regions_pipeline.bokeh_layout,
                &BindGroupEntries::sequential((
                    post_process.source,
                    &blur_regions_pipeline.sampler,
                    uniform_binding.clone(),
                    &red_green.default_view,
                    &blue.default_view,
                )),
            );
            run_blur_pass(
                &mut render_context,
                "blur_regions_bokeh_vertical_pass",
                vertical,
                &bind_group,
                uniform_offset,
                &[color_attachment(post_process.destination)],
            );
        }
    }
}

fn color_attachment(view: &TextureView) -> Option<RenderPassColorAttachment<'_>> {
    Some(RenderPassColorAttachment {
        view,
        resolve_target: None,
        depth_slice: None,
        ops: Operations::default(),
    })
}

fn run_blur_pass(
    render_context: &mut RenderContext,
    label: &'static str,
    pipeline: &RenderPipeline,
    bind_group: &BindGroup,
    uniform_offset: u32,
    color_attachments: &[Option<RenderPassColorAttachment>],
) {
    let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some(label),
        color_attachments,
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    render_pass.set_render_pipeline(pipeline);
    render_pass.set_bind_group(0, bind_group, &[uniform_offset]);
    render_pass.draw(0..3, 0..1);
}
