use bevy_ecs::{
    system::{Res, ResMut, SystemState},
    world::World,
};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipelineDescriptor, PipelineCache,
        RenderPipelineDescriptor, SpecializedComputePipeline, SpecializedComputePipelines,
        SpecializedMeshPipeline, SpecializedMeshPipelines, SpecializedRenderPipeline,
        SpecializedRenderPipelines,
    },
    renderer::RenderDevice,
};

use super::resource::IntoRenderResource;
//
// pub struct SpecializeRenderPipeline<'a, P: SpecializedRenderPipeline + 'static>(
//     pub &'a P,
//     pub P::Key,
// )
// where
//     <P as SpecializedRenderPipeline>::Key: Send + Sync;
//
// impl<'a, P: SpecializedRenderPipeline + 'static> IntoRenderResource
//     for SpecializeRenderPipeline<'a, P>
// where
//     <P as SpecializedRenderPipeline>::Key: Send + Sync,
// {
//     type Resource = CachedRenderPipelineId;
//
//     fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
//         let (mut specializer, pipelines) =
//             SystemState::<(ResMut<SpecializedRenderPipelines<P>>, Res<PipelineCache>)>::new(world)
//                 .get_mut(world);
//         specializer.specialize(&pipelines, self.0, self.1)
//     }
// }
//
// pub struct SpecializeComputePipeline<'a, P: SpecializedComputePipeline + 'static>(
//     pub &'a P,
//     pub P::Key,
// )
// where
//     <P as SpecializedComputePipeline>::Key: Send + Sync;
//
// impl<'a, P: SpecializedComputePipeline + 'static> IntoRenderResource
//     for SpecializeComputePipeline<'a, P>
// where
//     <P as SpecializedComputePipeline>::Key: Send + Sync,
// {
//     type Resource = CachedComputePipelineId;
//
//     fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
//         let (mut specializer, pipelines) =
//             SystemState::<(ResMut<SpecializedComputePipelines<P>>, Res<PipelineCache>)>::new(world)
//                 .get_mut(world);
//         specializer.specialize(&pipelines, self.0, self.1)
//     }
// }
//
// pub struct SpecializeMeshPipeline<'a, P: SpecializedMeshPipeline + 'static>(
//     pub &'a P,
//     pub P::Key,
//     pub &'a MeshVertexBufferLayoutRef,
// )
// where
//     <P as SpecializedMeshPipeline>::Key: Send + Sync;
//
// impl<'a, P: SpecializedMeshPipeline + 'static> IntoRenderResource for SpecializeMeshPipeline<'a, P>
// where
//     <P as SpecializedMeshPipeline>::Key: Send + Sync,
// {
//     type Resource = CachedRenderPipelineId;
//
//     fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
//         let (mut specializer, pipelines) =
//             SystemState::<(ResMut<SpecializedMeshPipelines<P>>, Res<PipelineCache>)>::new(world)
//                 .get_mut(world);
//         specializer
//             .specialize(&pipelines, self.0, self.1, self.2)
//             .unwrap()
//     }
// }

impl IntoRenderResource for RenderPipelineDescriptor {
    type Resource = CachedRenderPipelineId;

    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
        render_device.create_render_pipeline()
        let pipelines = world.resource::<PipelineCache>();
        pipelines.queue_render_pipeline(self)
    }
}

impl IntoRenderResource for ComputePipelineDescriptor {
    type Resource = CachedComputePipelineId;

    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
        let pipelines = world.resource::<PipelineCache>();
        pipelines.queue_compute_pipeline(self)
    }
}
