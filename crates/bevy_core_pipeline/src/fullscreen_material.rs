//! This is mostly a pluginified version of the `custom_post_processing` example
//!
//! The plugin will create a new node that runs a fullscreen triangle.
//!
//! Users need to use the [`FullscreenMaterial`] trait to define the parameters like the graph label or the graph ordering.

use core::marker::PhantomData;

use crate::FullscreenShader;
use bevy_app::{App, Plugin};
use bevy_asset::AssetServer;
use bevy_ecs::{
    component::Component,
    query::QueryItem,
    resource::Resource,
    system::{Commands, Res},
    world::World,
};
use bevy_image::BevyDefault;
use bevy_render::{
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_graph::{
        InternedRenderLabel, NodeRunError, RenderGraph, RenderGraphContext, RenderGraphError,
        RenderGraphExt, RenderLabel, RenderSubGraph, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        encase::internal::WriteInto,
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FragmentState, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderRef, ShaderStages, ShaderType, TextureFormat,
        TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice},
    view::ViewTarget,
    RenderApp, RenderStartup,
};
use bevy_utils::default;
use tracing::warn;

#[derive(Default)]
pub struct FullscreenMaterialPlugin<T: FullscreenMaterial> {
    _marker: PhantomData<T>,
}
impl<T: FullscreenMaterial> Plugin for FullscreenMaterialPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<T>::default(),
            UniformComponentPlugin::<T>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(RenderStartup, init_pipeline::<T>);

        render_app.add_render_graph_node::<ViewNodeRunner<FullscreenMaterialNode<T>>>(
            T::sub_graph(),
            T::node_label(),
        );
        // We can't use add_render_graph_edges because it doesn't accept a Vec<RenderLabel>
        if let Some(mut render_graph) = render_app.world_mut().get_resource_mut::<RenderGraph>()
            && let Some(graph) = render_graph.get_sub_graph_mut(T::sub_graph())
        {
            for window in T::node_edges().windows(2) {
                let [a, b] = window else {
                    break;
                };
                let Err(err) = graph.try_add_node_edge(*a, *b) else {
                    continue;
                };
                match err {
                    // Already existing edges are very easy to produce with this api
                    // and shouldn't cause a panic
                    RenderGraphError::EdgeAlreadyExists(_) => {}
                    _ => panic!("{err:?}"),
                }
            }
        } else {
            warn!("Failed to add edges for FullscreenMaterial");
        };
    }
}

/// A trait to define a material that will render to the entire screen using a fullscrene triangle
pub trait FullscreenMaterial:
    Component + ExtractComponent + Clone + Copy + ShaderType + WriteInto + Default
{
    /// The shader that will run on the entire screen using a fullscreen triangle
    fn fragment_shader() -> ShaderRef;
    /// The [`RenderSubGraph`] the effect will run in
    ///
    /// For 2d this is generally [`Core2d`] and for 3d it's [`Core3d`]
    fn sub_graph() -> impl RenderSubGraph;
    /// The label used to represent the render node that will run the pass
    fn node_label() -> impl RenderLabel;
    /// The list of `node_edges`. In 3d, for a post processing effect, it would look like this:
    ///
    /// ```
    /// vec![
    ///     Node3d::Tonemapping.intern(),
    ///     // Self::sub_graph().intern(), // <--- your own label here
    ///     Node3d::EndMainPassPostProcessing.intern(),
    /// ]
    /// ```
    ///
    /// This tell the render graph to run your fullscreen effect after the tonemapping pass but
    /// before the end of post processing. For 2d, it would be the same but using Node2d. You can
    /// specify any edges you want but make sure to include your own label.
    fn node_edges() -> Vec<InternedRenderLabel>;
}

#[derive(Resource)]
struct FullscreenMaterialPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
    pipeline_id_hdr: CachedRenderPipelineId,
}

fn init_pipeline<T: FullscreenMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = render_device.create_bind_group_layout(
        "post_process_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // The screen texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // The sampler that will be used to sample the screen texture
                sampler(SamplerBindingType::Filtering),
                // We use a uniform buffer so users can pass some data to the effect
                // Eventually we should just use a separate bind group for user data
                uniform_buffer::<T>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let shader = match T::fragment_shader() {
        ShaderRef::Default => {
            // TODO not sure what an actual fallback should be. An empty shader or output a solid
            // color to indicate a missing shader?
            unimplemented!("No default fallback for FullscreenMaterial shader")
        }
        ShaderRef::Handle(handle) => handle,
        ShaderRef::Path(path) => asset_server.load(path),
    };
    // Setup a fullscreen triangle for the vertex state.
    let vertex_state = fullscreen_shader.to_vertex_state();
    let mut desc = RenderPipelineDescriptor {
        label: Some("post_process_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    };
    let pipeline_id = pipeline_cache.queue_render_pipeline(desc.clone());
    desc.fragment.as_mut().unwrap().targets[0]
        .as_mut()
        .unwrap()
        .format = ViewTarget::TEXTURE_FORMAT_HDR;
    let pipeline_id_hdr = pipeline_cache.queue_render_pipeline(desc);
    commands.insert_resource(FullscreenMaterialPipeline {
        layout,
        sampler,
        pipeline_id,
        pipeline_id_hdr,
    });
}

#[derive(Default)]
struct FullscreenMaterialNode<T: FullscreenMaterial> {
    _marker: PhantomData<T>,
}

impl<T: FullscreenMaterial> ViewNode for FullscreenMaterialNode<T> {
    // TODO we should expose the depth buffer and the gbuffer if using deferred
    type ViewQuery = (&'static ViewTarget, &'static DynamicUniformIndex<T>);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let fullscreen_pipeline = world.resource::<FullscreenMaterialPipeline>();

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline_id = if view_target.is_hdr() {
            fullscreen_pipeline.pipeline_id_hdr
        } else {
            fullscreen_pipeline.pipeline_id
        };

        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
            return Ok(());
        };

        let data_uniforms = world.resource::<ComponentUniforms<T>>();
        let Some(settings_binding) = data_uniforms.uniforms().binding() else {
            return Ok(());
        };

        // We should maybe rename this because this can be used for other reasons that aren't
        // post-processing
        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "post_process_bind_group",
            &fullscreen_pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &fullscreen_pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("post_process_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
