//! Screen space reflections implemented via raymarching.

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    fullscreen_vertex_shader,
    prepass::{DeferredPrepass, DepthPrepass},
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
        binding_types, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, FragmentState,
        Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Shader, ShaderStages, ShaderType, SpecializedRenderPipeline,
        SpecializedRenderPipelines, TextureFormat, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::BevyDefault as _,
    view::{ExtractedView, Msaa, ViewTarget, ViewUniformOffset},
    Render, RenderApp, RenderSet,
};
use bevy_utils::prelude::default;

use crate::{
    binding_arrays_are_usable, graph::NodePbr, prelude::EnvironmentMapLight,
    MeshPipelineViewLayoutKey, MeshPipelineViewLayouts, MeshViewBindGroup, RenderViewLightProbes,
    ViewFogUniformOffset, ViewLightProbesUniformOffset, ViewLightsUniformOffset,
};

const SSR_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10438925299917978850);

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
#[derive(Clone, Copy, Component, Reflect, ShaderType)]
#[reflect(Component, Default)]
pub struct ScreenSpaceReflectionsSettings {
    /// The maximum PBR roughness level that will enable screen space
    /// reflections.
    pub perceptual_roughness_threshold: f32,

    /// An approximation value for the depth of objects in the depth buffer.
    pub thickness: f32,

    /// The maximum number of large raymarching steps that the SSR shader will
    /// perform in order to find reflected objects.
    ///
    /// Higher values result in higher-quality reflections, because the
    /// raymarching shader is less likely to miss objects. However, they take
    /// more GPU time.
    pub major_step_count: i32,

    /// The number of small steps that the SSR shader will perform in order to
    /// narrow down a more precise location for reflections.
    ///
    /// Higher values result in higher-quality reflections, especially when
    /// reproducing the edges of objects and textures on objects. However, they
    /// take more GPU time.
    pub minor_step_count: i32,
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
    bind_group_layout: BindGroupLayout,
    binding_arrays_are_usable: bool,
}

/// A GPU buffer that stores the screen space reflection settings for each view.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ScreenSpaceReflectionsBuffer(pub DynamicUniformBuffer<ScreenSpaceReflectionsSettings>);

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
    fn default() -> Self {
        Self {
            perceptual_roughness_threshold: 0.1,
            thickness: 1.0,
            major_step_count: 8,
            minor_step_count: 8,
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
            Some("SSR bind group"),
            &ssr_pipeline.bind_group_layout,
            &BindGroupEntries::single(postprocess.source),
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

        let bind_group_layout = render_device.create_bind_group_layout(
            "SSR bind group layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::FRAGMENT,
                binding_types::texture_2d(TextureSampleType::Float { filterable: true }),
            ),
        );

        Self {
            mesh_view_layouts,
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
        ),
        (
            With<ScreenSpaceReflectionsSettings>,
            With<DepthPrepass>,
            With<DeferredPrepass>,
        ),
    >,
) {
    for (entity, extracted_view, has_environment_maps) in &views {
        // SSR is only supported in the deferred pipeline, which has no MSAA
        // support. Thus we can assume MSAA is off.
        let mesh_pipeline_view_key = MeshPipelineViewLayoutKey::from(Msaa::Off)
            | MeshPipelineViewLayoutKey::DEPTH_PREPASS
            | MeshPipelineViewLayoutKey::DEFERRED_PREPASS;

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
    views: Query<(Entity, Option<&ScreenSpaceReflectionsSettings>), With<ExtractedView>>,
    mut ssr_settings_buffer: ResMut<ScreenSpaceReflectionsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let Some(mut writer) =
        ssr_settings_buffer.get_writer(views.iter().len(), &render_device, &render_queue)
    else {
        return;
    };

    for (view, ssr_settings) in views.iter() {
        let uniform_offset = match ssr_settings {
            None => 0,
            Some(ssr_settings) => writer.write(ssr_settings),
        };
        commands
            .entity(view)
            .insert(ViewScreenSpaceReflectionsUniformOffset(uniform_offset));
    }
}

impl ExtractComponent for ScreenSpaceReflectionsSettings {
    type QueryData = Read<ScreenSpaceReflectionsSettings>;

    type QueryFilter = ();

    type Out = ScreenSpaceReflectionsSettings;

    fn extract_component(settings: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(*settings)
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
            "SSR".into(),
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
