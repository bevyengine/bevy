use super::{prepare::PathtracerAccumulationTexture, Pathtracer};
use crate::scene::RaytracingSceneBindings;
use bevy_asset::load_embedded_asset;
use bevy_ecs::{
    query::QueryItem,
    world::{FromWorld, World},
};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        binding_types::{texture_storage_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId,
        ComputePassDescriptor, ComputePipelineDescriptor, ImageSubresourceRange, PipelineCache,
        ShaderStages, StorageTextureAccess, TextureFormat,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_utils::default;

pub mod graph {
    use bevy_render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub struct PathtracerNode;
}

pub struct PathtracerNode {
    bind_group_layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

impl ViewNode for PathtracerNode {
    type ViewQuery = (
        &'static Pathtracer,
        &'static PathtracerAccumulationTexture,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (pathtracer, accumulation_texture, camera, view_target, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let (Some(pipeline), Some(scene_bindings), Some(viewport), Some(view_uniforms)) = (
            pipeline_cache.get_compute_pipeline(self.pipeline),
            &scene_bindings.bind_group,
            camera.physical_viewport_size,
            view_uniforms.uniforms.binding(),
        ) else {
            return Ok(());
        };

        let bind_group = render_context.render_device().create_bind_group(
            "pathtracer_bind_group",
            &self.bind_group_layout,
            &BindGroupEntries::sequential((
                &accumulation_texture.0.default_view,
                view_target.get_unsampled_color_attachment().view,
                view_uniforms,
            )),
        );

        let command_encoder = render_context.command_encoder();

        if pathtracer.reset {
            command_encoder.clear_texture(
                &accumulation_texture.0.texture,
                &ImageSubresourceRange::default(),
            );
        }

        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("pathtracer"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, scene_bindings, &[]);
        pass.set_bind_group(1, &bind_group, &[view_uniform_offset.offset]);
        pass.dispatch_workgroups(viewport.x.div_ceil(8), viewport.y.div_ceil(8), 1);

        Ok(())
    }
}

impl FromWorld for PathtracerNode {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "pathtracer_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::ReadWrite),
                    texture_storage_2d(
                        ViewTarget::TEXTURE_FORMAT_HDR,
                        StorageTextureAccess::WriteOnly,
                    ),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("pathtracer_pipeline".into()),
            layout: vec![
                scene_bindings.bind_group_layout.clone(),
                bind_group_layout.clone(),
            ],
            shader: load_embedded_asset!(world, "pathtracer.wgsl"),
            ..default()
        });

        Self {
            bind_group_layout,
            pipeline,
        }
    }
}
