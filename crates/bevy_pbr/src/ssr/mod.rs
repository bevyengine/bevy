//! Screen space reflections implemented via raymarching.

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::{
    core_3d::{
        graph::{Core3d, Node3d},
        DEPTH_TEXTURE_SAMPLING_SUPPORTED,
    },
    fullscreen_vertex_shader,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    query::{Has, QueryItem, With},
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types, AddressMode, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, FilterMode,
        FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
        ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
        TextureFormat, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::BevyDefault as _,
    view::{ExtractedView, Msaa, ViewTarget, ViewUniformOffset},
    Render, RenderApp, RenderSet,
};
use bevy_utils::{info_once, prelude::default};

use crate::{
    binding_arrays_are_usable, graph::NodePbr, prelude::EnvironmentMapLight,
    MeshPipelineViewLayoutKey, MeshPipelineViewLayouts, MeshViewBindGroup, RenderViewLightProbes,
    ViewFogUniformOffset, ViewLightProbesUniformOffset, ViewLightsUniformOffset,
};

const SSR_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10438925299917978850);
const RAYMARCH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(8517409683450840946);

/// Enables screen-space reflections for a camera.
///
/// Screen-space reflections are currently only supported with deferred rendering.
pub struct ScreenSpaceReflectionsPlugin;

/// A convenient bundle to add screen space reflections to a camera, along with
/// the depth and deferred prepasses required to enable them.
#[derive(Bundle, Default)]
pub struct ScreenSpaceReflectionsBundle {
    /// The component that enables SSR.
    pub settings: ScreenSpaceReflectionsSettings,
    /// The depth prepass, needed for SSR.
    pub depth_prepass: DepthPrepass,
    /// The deferred prepass, needed for SSR.
    pub deferred_prepass: DeferredPrepass,
}

/// Add this component to a camera to enable *screen-space reflections* (SSR).
///
/// Screen-space reflections currently require deferred rendering in order to
/// appear. Therefore, you'll generally need to add a [`DepthPrepass`] and a
/// [`DeferredPrepass`] to the camera as well.
///
/// SSR currently performs no roughness filtering for glossy reflections, so
/// only very smooth surfaces will reflect objects in screen space. You can
/// adjust the `perceptual_roughness_threshold` in order to tune the threshold
/// below which screen-space reflections will be traced.
///
/// As with all screen-space techniques, SSR can only reflect objects on screen.
/// When objects leave the camera, they will disappear from reflections.
/// Alternatives that don't suffer from this problem include
/// [`crate::environment_map::ReflectionProbeBundle`]s. The advantage of SSR is
/// that it can reflect all objects, not just static ones.
///
/// SSR is an approximation technique and produces artifacts in some situations.
/// Hand-tuning the settings in this component will likely be useful.
///
/// Screen-space reflections are presently unsupported on WebGL 2 because of a
/// bug whereby Naga doesn't generate correct GLSL when sampling depth buffers,
/// which is required for screen-space raymarching.
#[derive(Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ScreenSpaceReflectionsSettings {
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

/// A version of [`ScreenSpaceReflectionsSettings`] for upload to the GPU.
///
/// For more information on these fields, see the corresponding documentation in
/// [`ScreenSpaceReflectionsSettings`].
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
        load_internal_asset!(app, SSR_SHADER_HANDLE, "ssr.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            RAYMARCH_SHADER_HANDLE,
            "raymarch.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ScreenSpaceReflectionsSettings>()
            .add_plugins(ExtractComponentPlugin::<ScreenSpaceReflectionsSettings>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ScreenSpaceReflectionsBuffer>()
            .add_systems(Render, prepare_ssr_pipelines.in_set(RenderSet::Prepare))
            .add_systems(
                Render,
                prepare_ssr_settings.in_set(RenderSet::PrepareResources),
            )
            .add_render_graph_node::<ViewNodeRunner<ScreenSpaceReflectionsNode>>(
                Core3d,
                NodePbr::ScreenSpaceReflections,
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<ScreenSpaceReflectionsPipeline>()
            .init_resource::<SpecializedRenderPipelines<ScreenSpaceReflectionsPipeline>>()
            .add_render_graph_edges(
                Core3d,
                (
                    NodePbr::DeferredLightingPass,
                    NodePbr::ScreenSpaceReflections,
                    Node3d::MainOpaquePass,
                ),
            );
    }
}

impl Default for ScreenSpaceReflectionsSettings {
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
            view_bind_group,
            ssr_pipeline_id,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Grab the render pipeline.
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(**ssr_pipeline_id) else {
            return Ok(());
        };

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
            label: Some("SSR pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: postprocess.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Set bind groups.
        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(
            0,
            &view_bind_group.value,
            &[
                view_uniform_offset.offset,
                view_lights_offset.offset,
                view_fog_offset.offset,
                **view_light_probes_offset,
                **view_ssr_offset,
            ],
        );

        // Perform the SSR render pass.
        render_pass.set_bind_group(1, &ssr_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

impl FromWorld for ScreenSpaceReflectionsPipeline {
    fn from_world(world: &mut World) -> Self {
        let mesh_view_layouts = world.resource::<MeshPipelineViewLayouts>().clone();
        let render_device = world.resource::<RenderDevice>();

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

        Self {
            mesh_view_layouts,
            color_sampler,
            depth_linear_sampler,
            depth_nearest_sampler,
            bind_group_layout,
            binding_arrays_are_usable: binding_arrays_are_usable(render_device),
        }
    }
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

impl ExtractComponent for ScreenSpaceReflectionsSettings {
    type QueryData = Read<ScreenSpaceReflectionsSettings>;

    type QueryFilter = ();

    type Out = ScreenSpaceReflectionsUniform;

    fn extract_component(settings: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        if !DEPTH_TEXTURE_SAMPLING_SUPPORTED {
            info_once!(
                "Disabling screen-space reflections on this platform because depth textures \
                aren't supported correctly"
            );
            return None;
        }

        Some((*settings).into())
    }
}

impl SpecializedRenderPipeline for ScreenSpaceReflectionsPipeline {
    type Key = ScreenSpaceReflectionsPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mesh_view_layout = self
            .mesh_view_layouts
            .get_view_layout(key.mesh_pipeline_view_key);

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
            layout: vec![mesh_view_layout.clone(), self.bind_group_layout.clone()],
            vertex: fullscreen_vertex_shader::fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: SSR_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.is_hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            push_constant_ranges: vec![],
            primitive: default(),
            depth_stencil: None,
            multisample: default(),
        }
    }
}

impl From<ScreenSpaceReflectionsSettings> for ScreenSpaceReflectionsUniform {
    fn from(settings: ScreenSpaceReflectionsSettings) -> Self {
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
