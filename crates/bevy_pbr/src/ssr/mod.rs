//! Screen space reflections implemented via raymarching.

use bevy_app::{App, Plugin};
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::{
    core_3d::{
        graph::{Core3d, Node3d},
        DEPTH_TEXTURE_SAMPLING_SUPPORTED,
    },
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    FullscreenShader,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::World,
};
use bevy_image::BevyDefault as _;
use bevy_light::EnvironmentMapLight;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{
        NodeRunError, RenderGraph, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types, AddressMode, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, FilterMode,
        FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
        ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
        TextureSampleType,
    },
    renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, Msaa, ViewTarget, ViewUniformOffset},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::{load_shader_library, Shader};
use bevy_utils::{once, prelude::default};
use tracing::info;

use crate::{
    binding_arrays_are_usable, graph::NodePbr, MeshPipelineViewLayoutKey, MeshPipelineViewLayouts,
    MeshViewBindGroup, RenderViewLightProbes, ViewEnvironmentMapUniformOffset,
    ViewFogUniformOffset, ViewLightProbesUniformOffset, ViewLightsUniformOffset,
};

/// Enables screen-space reflections for a camera.
///
/// Screen-space reflections are currently only supported with deferred rendering.
pub struct ScreenSpaceReflectionsPlugin;

/// Add this component to a camera to enable *screen-space reflections* (SSR).
///
/// Screen-space reflections currently require deferred rendering in order to
/// appear. Therefore, they also need the [`DepthPrepass`] and [`DeferredPrepass`]
/// components, which are inserted automatically.
///
/// SSR currently performs no roughness filtering for glossy reflections, so
/// only very smooth surfaces will reflect objects in screen space. You can
/// adjust the `perceptual_roughness_threshold` in order to tune the threshold
/// below which screen-space reflections will be traced.
///
/// As with all screen-space techniques, SSR can only reflect objects on screen.
/// When objects leave the camera, they will disappear from reflections.
/// An alternative that doesn't suffer from this problem is the combination of
/// a [`LightProbe`](bevy_light::LightProbe) and [`EnvironmentMapLight`]. The advantage of SSR is
/// that it can reflect all objects, not just static ones.
///
/// SSR is an approximation technique and produces artifacts in some situations.
/// Hand-tuning the settings in this component will likely be useful.
///
/// Screen-space reflections are presently unsupported on WebGL 2 because of a
/// bug whereby Naga doesn't generate correct GLSL when sampling depth buffers,
/// which is required for screen-space raymarching.
#[derive(Clone, Copy, Component, Reflect)]
#[reflect(Component, Default, Clone)]
#[require(DepthPrepass, DeferredPrepass)]
#[doc(alias = "Ssr")]
pub struct ScreenSpaceReflections {
    /// The maximum PBR roughness level that will enable screen space
    /// reflections.
    pub perceptual_roughness_threshold: f32,

    /// When marching the depth buffer, we only have 2.5D information and don't
    /// know how thick surfaces are. We shall assume that the depth buffer
    /// fragments are cuboids with a constant thickness defined by this
    /// parameter.
    pub thickness: f32,

    /// The number of steps to be taken at regular intervals to find an initial
    /// intersection. Must not be zero.
    ///
    /// Higher values result in higher-quality reflections, because the
    /// raymarching shader is less likely to miss objects. However, they take
    /// more GPU time.
    pub linear_steps: u32,

    /// Exponent to be applied in the linear part of the march.
    ///
    /// A value of 1.0 will result in equidistant steps, and higher values will
    /// compress the earlier steps, and expand the later ones. This might be
    /// desirable in order to get more detail close to objects.
    ///
    /// For optimal performance, this should be a small unsigned integer, such
    /// as 1 or 2.
    pub linear_march_exponent: f32,

    /// Number of steps in a bisection (binary search) to perform once the
    /// linear search has found an intersection. Helps narrow down the hit,
    /// increasing the chance of the secant method finding an accurate hit
    /// point.
    pub bisection_steps: u32,

    /// Approximate the root position using the secant method—by solving for
    /// line-line intersection between the ray approach rate and the surface
    /// gradient.
    pub use_secant: bool,
}

/// A version of [`ScreenSpaceReflections`] for upload to the GPU.
///
/// For more information on these fields, see the corresponding documentation in
/// [`ScreenSpaceReflections`].
#[derive(Clone, Copy, Component, ShaderType)]
pub struct ScreenSpaceReflectionsUniform {
    perceptual_roughness_threshold: f32,
    thickness: f32,
    linear_steps: u32,
    linear_march_exponent: f32,
    bisection_steps: u32,
    /// A boolean converted to a `u32`.
    use_secant: u32,
}

/// The node in the render graph that traces screen space reflections.
#[derive(Default)]
pub struct ScreenSpaceReflectionsNode;

/// Identifies which screen space reflections render pipeline a view needs.
#[derive(Component, Deref, DerefMut)]
pub struct ScreenSpaceReflectionsPipelineId(pub CachedRenderPipelineId);

/// Information relating to the render pipeline for the screen space reflections
/// shader.
#[derive(Resource)]
pub struct ScreenSpaceReflectionsPipeline {
    mesh_view_layouts: MeshPipelineViewLayouts,
    color_sampler: Sampler,
    depth_linear_sampler: Sampler,
    depth_nearest_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    binding_arrays_are_usable: bool,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

/// A GPU buffer that stores the screen space reflection settings for each view.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ScreenSpaceReflectionsBuffer(pub DynamicUniformBuffer<ScreenSpaceReflectionsUniform>);

/// A component that stores the offset within the
/// [`ScreenSpaceReflectionsBuffer`] for each view.
#[derive(Component, Default, Deref, DerefMut)]
pub struct ViewScreenSpaceReflectionsUniformOffset(u32);

/// Identifies a specific configuration of the SSR pipeline shader.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScreenSpaceReflectionsPipelineKey {
    mesh_pipeline_view_key: MeshPipelineViewLayoutKey,
    is_hdr: bool,
    has_environment_maps: bool,
}

impl Plugin for ScreenSpaceReflectionsPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "ssr.wgsl");
        load_shader_library!(app, "raymarch.wgsl");

        app.add_plugins(ExtractComponentPlugin::<ScreenSpaceReflections>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ScreenSpaceReflectionsBuffer>()
            .init_resource::<SpecializedRenderPipelines<ScreenSpaceReflectionsPipeline>>()
            .add_systems(
                RenderStartup,
                (
                    init_screen_space_reflections_pipeline,
                    add_screen_space_reflections_render_graph_edges,
                ),
            )
            .add_systems(Render, prepare_ssr_pipelines.in_set(RenderSystems::Prepare))
            .add_systems(
                Render,
                prepare_ssr_settings.in_set(RenderSystems::PrepareResources),
            )
            // Note: we add this node here but then we add edges in
            // `add_screen_space_reflections_render_graph_edges`.
            .add_render_graph_node::<ViewNodeRunner<ScreenSpaceReflectionsNode>>(
                Core3d,
                NodePbr::ScreenSpaceReflections,
            );
    }
}

fn add_screen_space_reflections_render_graph_edges(mut render_graph: ResMut<RenderGraph>) {
    let subgraph = render_graph.sub_graph_mut(Core3d);

    subgraph.add_node_edge(NodePbr::ScreenSpaceReflections, Node3d::MainOpaquePass);

    if subgraph
        .get_node_state(NodePbr::DeferredLightingPass)
        .is_ok()
    {
        subgraph.add_node_edge(
            NodePbr::DeferredLightingPass,
            NodePbr::ScreenSpaceReflections,
        );
    }
}

impl Default for ScreenSpaceReflections {
    // Reasonable default values.
    //
    // These are from
    // <https://gist.github.com/h3r2tic/9c8356bdaefbe80b1a22ae0aaee192db?permalink_comment_id=4552149#gistcomment-4552149>.
    fn default() -> Self {
        Self {
            perceptual_roughness_threshold: 0.1,
            linear_steps: 16,
            bisection_steps: 4,
            use_secant: true,
            thickness: 0.25,
            linear_march_exponent: 1.0,
        }
    }
}

impl ViewNode for ScreenSpaceReflectionsNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<ViewFogUniformOffset>,
        Read<ViewLightProbesUniformOffset>,
        Read<ViewScreenSpaceReflectionsUniformOffset>,
        Read<ViewEnvironmentMapUniformOffset>,
        Read<MeshViewBindGroup>,
        Read<ScreenSpaceReflectionsPipelineId>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            view_target,
            view_uniform_offset,
            view_lights_offset,
            view_fog_offset,
            view_light_probes_offset,
            view_ssr_offset,
            view_environment_map_offset,
            view_bind_group,
            ssr_pipeline_id,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Grab the render pipeline.
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(**ssr_pipeline_id) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        // Set up a standard pair of postprocessing textures.
        let postprocess = view_target.post_process_write();

        // Create the bind group for this view.
        let ssr_pipeline = world.resource::<ScreenSpaceReflectionsPipeline>();
        let ssr_bind_group = render_context.render_device().create_bind_group(
            "SSR bind group",
            &ssr_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                postprocess.source,
                &ssr_pipeline.color_sampler,
                &ssr_pipeline.depth_linear_sampler,
                &ssr_pipeline.depth_nearest_sampler,
            )),
        );

        // Build the SSR render pass.
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("ssr"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: postprocess.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let pass_span = diagnostics.pass_span(&mut render_pass, "ssr");

        // Set bind groups.
        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(
            0,
            &view_bind_group.main,
            &[
                view_uniform_offset.offset,
                view_lights_offset.offset,
                view_fog_offset.offset,
                **view_light_probes_offset,
                **view_ssr_offset,
                **view_environment_map_offset,
            ],
        );
        render_pass.set_bind_group(1, &view_bind_group.binding_array, &[]);

        // Perform the SSR render pass.
        render_pass.set_bind_group(2, &ssr_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_pass);

        Ok(())
    }
}

pub fn init_screen_space_reflections_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    mesh_view_layouts: Res<MeshPipelineViewLayouts>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    // Create the bind group layout.
    let bind_group_layout = render_device.create_bind_group_layout(
        "SSR bind group layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                binding_types::texture_2d(TextureSampleType::Float { filterable: true }),
                binding_types::sampler(SamplerBindingType::Filtering),
                binding_types::sampler(SamplerBindingType::Filtering),
                binding_types::sampler(SamplerBindingType::NonFiltering),
            ),
        ),
    );

    // Create the samplers we need.

    let color_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: "SSR color sampler".into(),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });

    let depth_linear_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: "SSR depth linear sampler".into(),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });

    let depth_nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
        label: "SSR depth nearest sampler".into(),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        ..default()
    });

    commands.insert_resource(ScreenSpaceReflectionsPipeline {
        mesh_view_layouts: mesh_view_layouts.clone(),
        color_sampler,
        depth_linear_sampler,
        depth_nearest_sampler,
        bind_group_layout,
        binding_arrays_are_usable: binding_arrays_are_usable(&render_device, &render_adapter),
        fullscreen_shader: fullscreen_shader.clone(),
        // Even though ssr was loaded using load_shader_library, we can still access it like a
        // normal embedded asset (so we can use it as both a library or a kernel).
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "ssr.wgsl"),
    });
}

/// Sets up screen space reflection pipelines for each applicable view.
pub fn prepare_ssr_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<ScreenSpaceReflectionsPipeline>>,
    ssr_pipeline: Res<ScreenSpaceReflectionsPipeline>,
    views: Query<
        (
            Entity,
            &ExtractedView,
            Has<RenderViewLightProbes<EnvironmentMapLight>>,
            Has<NormalPrepass>,
            Has<MotionVectorPrepass>,
        ),
        (
            With<ScreenSpaceReflectionsUniform>,
            With<DepthPrepass>,
            With<DeferredPrepass>,
        ),
    >,
) {
    for (
        entity,
        extracted_view,
        has_environment_maps,
        has_normal_prepass,
        has_motion_vector_prepass,
    ) in &views
    {
        // SSR is only supported in the deferred pipeline, which has no MSAA
        // support. Thus we can assume MSAA is off.
        let mut mesh_pipeline_view_key = MeshPipelineViewLayoutKey::from(Msaa::Off)
            | MeshPipelineViewLayoutKey::DEPTH_PREPASS
            | MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
        mesh_pipeline_view_key.set(
            MeshPipelineViewLayoutKey::NORMAL_PREPASS,
            has_normal_prepass,
        );
        mesh_pipeline_view_key.set(
            MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS,
            has_motion_vector_prepass,
        );

        // Build the pipeline.
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &ssr_pipeline,
            ScreenSpaceReflectionsPipelineKey {
                mesh_pipeline_view_key,
                is_hdr: extracted_view.hdr,
                has_environment_maps,
            },
        );

        // Note which pipeline ID was used.
        commands
            .entity(entity)
            .insert(ScreenSpaceReflectionsPipelineId(pipeline_id));
    }
}

/// Gathers up screen space reflection settings for each applicable view and
/// writes them into a GPU buffer.
pub fn prepare_ssr_settings(
    mut commands: Commands,
    views: Query<(Entity, Option<&ScreenSpaceReflectionsUniform>), With<ExtractedView>>,
    mut ssr_settings_buffer: ResMut<ScreenSpaceReflectionsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let Some(mut writer) =
        ssr_settings_buffer.get_writer(views.iter().len(), &render_device, &render_queue)
    else {
        return;
    };

    for (view, ssr_uniform) in views.iter() {
        let uniform_offset = match ssr_uniform {
            None => 0,
            Some(ssr_uniform) => writer.write(ssr_uniform),
        };
        commands
            .entity(view)
            .insert(ViewScreenSpaceReflectionsUniformOffset(uniform_offset));
    }
}

impl ExtractComponent for ScreenSpaceReflections {
    type QueryData = Read<ScreenSpaceReflections>;

    type QueryFilter = ();

    type Out = ScreenSpaceReflectionsUniform;

    fn extract_component(settings: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        if !DEPTH_TEXTURE_SAMPLING_SUPPORTED {
            once!(info!(
                "Disabling screen-space reflections on this platform because depth textures \
                aren't supported correctly"
            ));
            return None;
        }

        Some((*settings).into())
    }
}

impl SpecializedRenderPipeline for ScreenSpaceReflectionsPipeline {
    type Key = ScreenSpaceReflectionsPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let layout = self
            .mesh_view_layouts
            .get_view_layout(key.mesh_pipeline_view_key);
        let layout = vec![
            layout.main_layout.clone(),
            layout.binding_array_layout.clone(),
            self.bind_group_layout.clone(),
        ];

        let mut shader_defs = vec![
            "DEPTH_PREPASS".into(),
            "DEFERRED_PREPASS".into(),
            "SCREEN_SPACE_REFLECTIONS".into(),
        ];

        if key.has_environment_maps {
            shader_defs.push("ENVIRONMENT_MAP".into());
        }

        if self.binding_arrays_are_usable {
            shader_defs.push("MULTIPLE_LIGHT_PROBES_IN_ARRAY".into());
        }

        RenderPipelineDescriptor {
            label: Some("SSR pipeline".into()),
            layout,
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: if key.is_hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

impl From<ScreenSpaceReflections> for ScreenSpaceReflectionsUniform {
    fn from(settings: ScreenSpaceReflections) -> Self {
        Self {
            perceptual_roughness_threshold: settings.perceptual_roughness_threshold,
            thickness: settings.thickness,
            linear_steps: settings.linear_steps,
            linear_march_exponent: settings.linear_march_exponent,
            bisection_steps: settings.bisection_steps,
            use_secant: settings.use_secant as u32,
        }
    }
}
