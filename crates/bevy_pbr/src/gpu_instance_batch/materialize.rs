use bevy_app::App;
use bevy_asset::{embedded_asset, load_embedded_asset};
use bevy_ecs::prelude::*;
use bevy_math::UVec2;
use bevy_render::{
    batching::gpu_preprocessing::BatchedInstanceBuffers,
    diagnostic::RecordDiagnostics as _,
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only, uniform_buffer},
        BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BufferId,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache,
        ShaderStages, ShaderType, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue},
    sync_world::MainEntityHashMap,
    GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};

use super::{
    GpuInstanceBatchReservation, GpuInstanceBatchReservations, GpuInstanceBatchSystems,
    GpuMeshInstance,
};
use crate::{MeshCullingData, MeshCullingDataBuffer, MeshInputUniform, MeshUniform};

#[derive(Clone, Copy, Default, ShaderType)]
struct MaterializationMetadata {
    template: MeshInputUniform,
    bounds: MeshCullingData,
    input_base: u32,
    capacity: u32,
    pad: UVec2,
}

#[derive(Default)]
pub(super) struct MaterializationJob {
    metadata: UniformBuffer<MaterializationMetadata>,
    bind_group: Option<BindGroup>,
    bound_buffers: Option<[BufferId; 3]>,
    pub(super) active: bool,
}

impl MaterializationJob {
    pub fn update(
        &mut self,
        template: MeshInputUniform,
        bounds: MeshCullingData,
        input_base: u32,
        capacity: u32,
    ) {
        self.metadata.set(MaterializationMetadata {
            template,
            bounds,
            input_base,
            capacity,
            pad: UVec2::ZERO,
        });
        self.active = true;
    }
}

#[derive(Resource, Default)]
pub(crate) struct GpuInstanceMaterialization {
    pub(super) jobs: MainEntityHashMap<MaterializationJob>,
}

#[derive(Resource)]
pub(crate) struct MaterializationPipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
}

pub(super) fn build(app: &mut App) {
    embedded_asset!(app, "materialize.wesl");
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .init_gpu_resource::<GpuInstanceMaterialization>()
        .add_systems(RenderStartup, init_pipeline)
        .add_systems(
            Render,
            prepare_bind_groups
                .in_set(RenderSystems::PrepareBindGroups)
                .run_if(resource_exists::<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>)
                .run_if(resource_exists::<MaterializationPipeline>),
        )
        .add_systems(
            RenderGraph,
            materialize
                .in_set(GpuInstanceBatchSystems::Materialize)
                .run_if(resource_exists::<MaterializationPipeline>),
        );
}

fn init_pipeline(
    mut commands: Commands,
    cache: Res<PipelineCache>,
    assets: Res<bevy_asset::AssetServer>,
    support: Res<bevy_render::batching::gpu_preprocessing::GpuPreprocessingSupport>,
) {
    if !support.is_culling_supported() {
        return;
    }
    let layout = BindGroupLayoutDescriptor::new(
        "GPU instance materialization",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<MaterializationMetadata>(false),
                storage_buffer_read_only::<GpuMeshInstance>(false),
                storage_buffer::<MeshInputUniform>(false),
                storage_buffer::<MeshCullingData>(false),
            ),
        ),
    );
    let pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("GPU instance materialization".into()),
        layout: vec![layout.clone()],
        shader: load_embedded_asset!(assets.as_ref(), "materialize.wesl"),
        ..Default::default()
    });
    commands.insert_resource(MaterializationPipeline { layout, pipeline });
}

pub(super) fn pipelines_ready(
    pipeline: Option<&MaterializationPipeline>,
    range_pipeline: Option<CachedComputePipelineId>,
    cache: &PipelineCache,
) -> bool {
    pipeline.is_some_and(|pipeline| cache.get_compute_pipeline(pipeline.pipeline).is_some())
        && range_pipeline.is_some_and(|id| cache.get_compute_pipeline(id).is_some())
}

fn prepare_bind_groups(
    mut materialization: ResMut<GpuInstanceMaterialization>,
    reservations: Res<GpuInstanceBatchReservations>,
    buffers: Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    culling: Res<MeshCullingDataBuffer>,
    pipeline: Res<MaterializationPipeline>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let (Some(input), Some(culling)) = (
        buffers.current_input_buffer.buffer().buffer(),
        culling.buffer(),
    ) else {
        return;
    };
    for (entity, job) in &mut materialization.jobs {
        if !job.active {
            continue;
        }
        let Some(output) = reservations
            .get(*entity)
            .map(GpuInstanceBatchReservation::output_buffer)
        else {
            continue;
        };
        job.metadata.write_buffer(&device, &queue);
        let buffers = [output.id(), input.id(), culling.id()];
        if job.bound_buffers == Some(buffers) {
            continue;
        }
        job.bound_buffers = Some(buffers);
        job.bind_group = Some(device.create_bind_group(
            "GPU instance materialization",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                job.metadata.binding().unwrap(),
                output.as_entire_binding(),
                input.as_entire_binding(),
                culling.as_entire_binding(),
            )),
        ));
    }
}

fn materialize(
    mut render_context: RenderContext,
    materialization: Res<GpuInstanceMaterialization>,
    pipeline: Res<MaterializationPipeline>,
    cache: Res<PipelineCache>,
) {
    if !materialization.jobs.values().any(|job| job.active) {
        return;
    }
    let Some(pipeline) = cache.get_compute_pipeline(pipeline.pipeline) else {
        return;
    };
    let diagnostics = render_context.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("GPU instance materialization"),
            timestamp_writes: None,
        });
    let pass_span = diagnostics.pass_span(&mut pass, "gpu_instance_materialization");
    pass.set_pipeline(pipeline);
    for job in materialization.jobs.values() {
        if !job.active {
            continue;
        }
        let Some(bind_group) = &job.bind_group else {
            continue;
        };
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(job.metadata.get().capacity.div_ceil(64), 1, 1);
    }
    pass_span.end(&mut pass);
}
