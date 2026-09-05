use super::{
    allocator::RetainedBindingArray, lights::GpuLightSource, RaytracingSceneBindings,
    TlasInstanceSetupPipeline,
};
use bevy_ecs::system::{Res, ResMut};
use bevy_pbr::DfgLut;
use bevy_render::{
    render_asset::RenderAssets,
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupLayout, Buffer, BufferBinding, BufferDescriptor,
        BufferId, BufferSize, BufferUsages, PipelineCache, Sampler, SamplerId,
        SparseBufferUpdateBindGroups, SparseBufferUpdateJobs, SparseBufferUpdatePipelines,
        TextureView, TextureViewId,
    },
    renderer::RenderDevice,
    texture::{FallbackImage, GpuImage},
};
use core::{mem::size_of, ops::Deref};
use tracing::info_span;

pub struct BindGroupCacheState {
    cached: [Option<BindGroup>; 2],
    pub invalid: bool,
    last_buffer_ids: [Option<BufferId>; 9],
    last_light_count: u32,
    last_dfg_ids: Option<(TextureViewId, SamplerId)>,
    pub dummy_buffer: Buffer,
}

impl BindGroupCacheState {
    pub fn new(render_device: &RenderDevice) -> Self {
        // Binding arrays are dense, so freed slots still need something valid bound into them
        let dummy_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_dummy_binding_array_buffer"),
            size: 48,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        Self {
            cached: [None, None],
            invalid: true,
            last_buffer_ids: [None; 9],
            last_light_count: 0,
            last_dfg_ids: None,
            dummy_buffer,
        }
    }
}

fn buffer_bindings<'a>(
    buffers: &'a RetainedBindingArray<BufferId, Buffer>,
    dummy: &'a Buffer,
) -> Vec<BufferBinding<'a>> {
    buffers
        .iter()
        .map(|buffer| buffer.unwrap_or(dummy).as_entire_buffer_binding())
        .collect()
}

impl RaytracingSceneBindings {
    /// Each sparse buffer's GPU buffer id, or `None` where it has not been created yet.
    fn buffer_ids(&self) -> [Option<BufferId>; 9] {
        [
            self.assets.materials.buffer().map(Buffer::id),
            self.instances.transforms.buffer().map(Buffer::id),
            self.instances
                .previous_frame_transforms
                .buffer()
                .map(Buffer::id),
            self.instances.geometry_ids.buffer().map(Buffer::id),
            self.instances.material_ids.buffer().map(Buffer::id),
            self.instances.blas_refs.buffer().map(Buffer::id),
            self.lights.sources.buffer().map(Buffer::id),
            self.lights.directional_lights.buffer().map(Buffer::id),
            self.lights
                .previous_frame_id_translations
                .buffer()
                .map(Buffer::id),
        ]
    }

    fn take_bind_group_invalidation(
        &mut self,
        dfg_view: &TextureView,
        dfg_sampler: &Sampler,
    ) -> bool {
        let mut invalid = self.bind_groups.invalid;
        self.bind_groups.invalid = false;

        for dirty in [
            &mut self.instances.vertex_buffers.dirty,
            &mut self.instances.index_buffers.dirty,
            &mut self.assets.textures.dirty,
        ] {
            invalid |= core::mem::replace(dirty, false);
        }

        let buffer_ids = self.buffer_ids();
        if self.bind_groups.last_buffer_ids != buffer_ids {
            self.bind_groups.last_buffer_ids = buffer_ids;
            invalid = true;
        }

        let light_count = self.lights.index.len() as u32;
        if self.bind_groups.last_light_count != light_count {
            self.bind_groups.last_light_count = light_count;
            invalid = true;
        }

        let dfg_ids = Some((dfg_view.id(), dfg_sampler.id()));
        if self.bind_groups.last_dfg_ids != dfg_ids {
            self.bind_groups.last_dfg_ids = dfg_ids;
            invalid = true;
        }

        invalid
    }

    fn create_bind_group(
        &self,
        current_index: usize,
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
        fallback_texture: &FallbackImage,
        dfg_view: &TextureView,
        dfg_sampler: &Sampler,
    ) -> BindGroup {
        let _span = info_span!("create_bind_group").entered();
        let dummy = &self.bind_groups.dummy_buffer;
        let vertex_buffers = buffer_bindings(&self.instances.vertex_buffers, dummy);
        let index_buffers = buffer_bindings(&self.instances.index_buffers, dummy);

        let (mut textures, mut samplers): (Vec<_>, Vec<_>) = self
            .assets
            .textures
            .iter()
            .map(|texture| match texture {
                Some((view, sampler)) => (view.deref(), sampler.deref()),
                None => (
                    fallback_texture.d2.texture_view.deref(),
                    fallback_texture.d2.sampler.deref(),
                ),
            })
            .unzip();
        if textures.is_empty() {
            textures.push(fallback_texture.d2.texture_view.deref());
            samplers.push(fallback_texture.d2.sampler.deref());
        }

        let light_sources = BufferBinding {
            buffer: self.lights.sources.buffer().unwrap(),
            offset: 0,
            size: BufferSize::new(
                self.lights.index.len() as u64 * size_of::<GpuLightSource>() as u64,
            ),
        };

        let materials = self
            .assets
            .materials
            .buffer()
            .unwrap()
            .as_entire_buffer_binding();
        let transforms = self
            .instances
            .transforms
            .buffer()
            .unwrap()
            .as_entire_buffer_binding();
        let previous_frame_transforms = self
            .instances
            .previous_frame_transforms
            .buffer()
            .unwrap()
            .as_entire_buffer_binding();
        let geometry_ids = self
            .instances
            .geometry_ids
            .buffer()
            .unwrap()
            .as_entire_buffer_binding();
        let material_ids = self
            .instances
            .material_ids
            .buffer()
            .unwrap()
            .as_entire_buffer_binding();
        let directional_lights = self
            .lights
            .directional_lights
            .buffer()
            .unwrap()
            .as_entire_buffer_binding();
        let translations = self
            .lights
            .previous_frame_id_translations
            .buffer()
            .unwrap()
            .as_entire_buffer_binding();

        let current = self.tlas.structures[current_index].as_ref().unwrap();
        let previous = self.tlas.structures[current_index ^ 1]
            .as_ref()
            .filter(|_| self.tlas.built[current_index ^ 1])
            .unwrap_or(current);

        render_device.create_bind_group(
            "raytracing_scene_bind_group",
            layout,
            &BindGroupEntries::sequential((
                vertex_buffers.as_slice(),
                index_buffers.as_slice(),
                textures.as_slice(),
                samplers.as_slice(),
                materials,
                current.as_binding(),
                previous.as_binding(),
                transforms,
                previous_frame_transforms,
                geometry_ids,
                material_ids,
                light_sources,
                directional_lights,
                translations,
                dfg_view,
                dfg_sampler,
            )),
        )
    }

    fn cache_bind_group(
        &mut self,
        current_index: usize,
        render_device: &RenderDevice,
        pipeline_cache: &PipelineCache,
        fallback_texture: &FallbackImage,
        dfg_view: &TextureView,
        dfg_sampler: &Sampler,
    ) -> BindGroup {
        if let Some(bind_group) = &self.bind_groups.cached[current_index] {
            return bind_group.clone();
        }

        // Only resolve the layout on a miss, as it locks and hashes the whole descriptor
        let layout = pipeline_cache.get_bind_group_layout(&self.bind_group_layout);
        let bind_group = self.create_bind_group(
            current_index,
            render_device,
            &layout,
            fallback_texture,
            dfg_view,
            dfg_sampler,
        );
        if self.tlas.previous_binding_is_stable() {
            self.bind_groups.cached[current_index] = Some(bind_group.clone());
        }
        bind_group
    }
}

/// Finalizes sparse uploads and selects the cached bind group for this frame's TLAS.
pub fn prepare_raytracing_scene_bind_group(
    texture_assets: Res<RenderAssets<GpuImage>>,
    fallback_texture: Res<FallbackImage>,
    dfg_lut: Res<DfgLut>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    sparse_buffer_update_pipelines: Res<SparseBufferUpdatePipelines>,
    instance_setup_pipeline: Res<TlasInstanceSetupPipeline>,
    mut sparse_buffer_update_jobs: ResMut<SparseBufferUpdateJobs>,
    mut sparse_buffer_update_bind_groups: ResMut<SparseBufferUpdateBindGroups>,
    mut bindings: ResMut<RaytracingSceneBindings>,
) {
    let bindings = &mut *bindings;

    prepare_sparse_uploads(
        bindings,
        &render_device,
        &pipeline_cache,
        &mut sparse_buffer_update_jobs,
        &mut sparse_buffer_update_bind_groups,
        &sparse_buffer_update_pipelines,
    );

    bindings.tlas.update_instance_setup_bind_group(
        &bindings.instances,
        &render_device,
        &pipeline_cache,
        &instance_setup_pipeline,
    );
    bindings.bind_group = None;

    if bindings.instances.live_count == 0 || bindings.lights.index.is_empty() {
        return;
    }
    if bindings.tlas.structures[bindings.tlas.current_index].is_none() {
        return;
    }

    let (dfg_view, dfg_sampler) = texture_assets
        .get(&dfg_lut.texture)
        .map(|image| (&image.texture_view, &image.sampler))
        .unwrap_or((
            &fallback_texture.d2.texture_view,
            &fallback_texture.d2.sampler,
        ));

    if bindings.take_bind_group_invalidation(dfg_view, dfg_sampler) {
        bindings.bind_groups.cached = [None, None];
    }

    let current_index = bindings.tlas.current_index;
    bindings.bind_group = Some(bindings.cache_bind_group(
        current_index,
        &render_device,
        &pipeline_cache,
        &fallback_texture,
        dfg_view,
        dfg_sampler,
    ));
}

/// Queues the compute jobs that scatter each buffer's staged elements into its GPU buffer.
fn prepare_sparse_uploads(
    bindings: &mut RaytracingSceneBindings,
    device: &RenderDevice,
    cache: &PipelineCache,
    jobs: &mut SparseBufferUpdateJobs,
    groups: &mut SparseBufferUpdateBindGroups,
    pipelines: &SparseBufferUpdatePipelines,
) {
    let _span = info_span!("prepare_sparse_uploads").entered();

    bindings
        .assets
        .materials
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);

    let instances = &mut bindings.instances;
    instances
        .transforms
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
    instances
        .previous_frame_transforms
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
    instances
        .geometry_ids
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
    instances
        .material_ids
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
    if bindings.tlas.uses_raw_build() {
        instances
            .blas_refs
            .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
    }

    let lights = &mut bindings.lights;
    lights
        .sources
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
    lights
        .directional_lights
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
    lights
        .previous_frame_id_translations
        .prepare_to_populate_buffers(device, cache, jobs, groups, pipelines);
}
