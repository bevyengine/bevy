use super::{persistent_buffer::PersistentGpuBuffer, Meshlet, MeshletBoundingSphere, MeshletMesh};
use crate::{
    Material, MeshFlags, MeshTransforms, MeshUniform, NotShadowCaster, NotShadowReceiver,
    PreviousGlobalTransform, RenderMaterialInstances, ShadowView,
};
use bevy_asset::{AssetEvent, AssetId, AssetServer, Assets, Handle, UntypedAssetId};
use bevy_core_pipeline::core_3d::Camera3d;
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    event::EventReader,
    query::{AnyOf, Has},
    system::{Commands, Query, Res, ResMut, Resource, SystemState},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, RenderLayers, ViewDepthTexture, ViewUniform, ViewUniforms},
    MainWorld,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{default, HashMap, HashSet};
use encase::internal::WriteInto;
use std::{
    iter,
    mem::size_of,
    ops::{DerefMut, Range},
    sync::Arc,
};

/// Create and queue for uploading to the GPU [`MeshUniform`] components for
/// [`MeshletMesh`] entities, as well as queuing uploads for any new meshlet mesh
/// assets that have not already been uploaded to the GPU.
pub fn extract_meshlet_meshes(
    // TODO: Replace main_world when Extract<ResMut<Assets<MeshletMesh>>> is possible
    mut main_world: ResMut<MainWorld>,
    mut gpu_scene: ResMut<MeshletGpuScene>,
) {
    let mut system_state: SystemState<(
        Query<(
            Entity,
            &Handle<MeshletMesh>,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            Option<&RenderLayers>,
            Has<NotShadowReceiver>,
            Has<NotShadowCaster>,
        )>,
        Res<AssetServer>,
        ResMut<Assets<MeshletMesh>>,
        EventReader<AssetEvent<MeshletMesh>>,
    )> = SystemState::new(&mut main_world);
    let (instances_query, asset_server, mut assets, mut asset_events) =
        system_state.get_mut(&mut main_world);

    // Reset all temporary data for MeshletGpuScene
    gpu_scene.reset();

    // Free GPU buffer space for any modified or dropped MeshletMesh assets
    for asset_event in asset_events.read() {
        if let AssetEvent::Unused { id } | AssetEvent::Modified { id } = asset_event {
            if let Some((
                [vertex_data_slice, vertex_ids_slice, indices_slice, meshlets_slice, meshlet_bounding_spheres_slice],
                _,
            )) = gpu_scene.meshlet_mesh_slices.remove(id)
            {
                gpu_scene.vertex_data.mark_slice_unused(vertex_data_slice);
                gpu_scene.vertex_ids.mark_slice_unused(vertex_ids_slice);
                gpu_scene.indices.mark_slice_unused(indices_slice);
                gpu_scene.meshlets.mark_slice_unused(meshlets_slice);
                gpu_scene
                    .meshlet_bounding_spheres
                    .mark_slice_unused(meshlet_bounding_spheres_slice);
            }
        }
    }

    for (
        instance_index,
        (
            instance,
            handle,
            transform,
            previous_transform,
            render_layers,
            not_shadow_receiver,
            not_shadow_caster,
        ),
    ) in instances_query.iter().enumerate()
    {
        // Skip instances with an unloaded MeshletMesh asset
        if asset_server.is_managed(handle.id())
            && !asset_server.is_loaded_with_dependencies(handle.id())
        {
            continue;
        }

        // Upload the instance's MeshletMesh asset data, if not done already, along with other per-frame per-instance data.
        gpu_scene.queue_meshlet_mesh_upload(
            instance,
            render_layers.cloned().unwrap_or(default()),
            not_shadow_caster,
            handle,
            &mut assets,
            instance_index as u32,
        );

        // Build a MeshUniform for each instance
        let transform = transform.affine();
        let previous_transform = previous_transform.map(|t| t.0).unwrap_or(transform);
        let mut flags = if not_shadow_receiver {
            MeshFlags::empty()
        } else {
            MeshFlags::SHADOW_RECEIVER
        };
        if transform.matrix3.determinant().is_sign_positive() {
            flags |= MeshFlags::SIGN_DETERMINANT_MODEL_3X3;
        }
        let transforms = MeshTransforms {
            transform: (&transform).into(),
            previous_transform: (&previous_transform).into(),
            flags: flags.bits(),
        };
        gpu_scene
            .instance_uniforms
            .get_mut()
            .push(MeshUniform::new(&transforms, None));
    }
}

/// Upload all newly queued [`MeshletMesh`] asset data from [`extract_meshlet_meshes`] to the GPU.
pub fn perform_pending_meshlet_mesh_writes(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    render_queue: Res<RenderQueue>,
    render_device: Res<RenderDevice>,
) {
    gpu_scene
        .vertex_data
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .vertex_ids
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .indices
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .meshlets
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .meshlet_bounding_spheres
        .perform_writes(&render_queue, &render_device);
}

/// For each entity in the scene, record what material ID (for use with depth testing during the meshlet mesh material draw nodes)
/// its material was assigned in the `prepare_material_meshlet_meshes` systems, and note that the material is used by at least one entity in the scene.
pub fn queue_material_meshlet_meshes<M: Material>(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
) {
    // TODO: Ideally we could parallelize this system, both between different materials, and the loop over instances
    let gpu_scene = gpu_scene.deref_mut();

    for (i, (instance, _, _)) in gpu_scene.instances.iter().enumerate() {
        if let Some(material_asset_id) = render_material_instances.get(instance) {
            let material_asset_id = material_asset_id.untyped();
            if let Some(material_id) = gpu_scene.material_id_lookup.get(&material_asset_id) {
                gpu_scene.material_ids_present_in_scene.insert(*material_id);
                gpu_scene.instance_material_ids.get_mut()[i] = *material_id;
            }
        }
    }
}

// TODO: Try using Queue::write_buffer_with() in queue_meshlet_mesh_upload() to reduce copies
fn upload_storage_buffer<T: ShaderSize + bytemuck::Pod>(
    buffer: &mut StorageBuffer<Vec<T>>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
) where
    Vec<T>: WriteInto,
{
    let inner = buffer.buffer();
    let capacity = inner.map_or(0, |b| b.size());
    let size = buffer.get().size().get() as BufferAddress;

    if capacity >= size {
        let inner = inner.unwrap();
        let bytes = bytemuck::cast_slice(buffer.get().as_slice());
        render_queue.write_buffer(inner, 0, bytes);
    } else {
        buffer.write_buffer(render_device, render_queue);
    }
}

pub fn prepare_meshlet_per_frame_resources(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    views: Query<(
        Entity,
        &ExtractedView,
        Option<&RenderLayers>,
        AnyOf<(&Camera3d, &ShadowView)>,
    )>,
    mut texture_cache: ResMut<TextureCache>,
    render_queue: Res<RenderQueue>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    gpu_scene
        .previous_cluster_id_starts
        .retain(|_, (_, active)| *active);

    if gpu_scene.scene_meshlet_count == 0 {
        return;
    }

    let gpu_scene = gpu_scene.as_mut();

    gpu_scene
        .instance_uniforms
        .write_buffer(&render_device, &render_queue);
    upload_storage_buffer(
        &mut gpu_scene.instance_material_ids,
        &render_device,
        &render_queue,
    );
    upload_storage_buffer(
        &mut gpu_scene.thread_instance_ids,
        &render_device,
        &render_queue,
    );
    upload_storage_buffer(
        &mut gpu_scene.thread_meshlet_ids,
        &render_device,
        &render_queue,
    );
    upload_storage_buffer(
        &mut gpu_scene.previous_cluster_ids,
        &render_device,
        &render_queue,
    );

    let needed_buffer_size = 4 * gpu_scene.scene_triangle_count;
    let visibility_buffer_draw_index_buffer =
        match &mut gpu_scene.visibility_buffer_draw_index_buffer {
            Some(buffer) if buffer.size() >= needed_buffer_size => buffer.clone(),
            slot => {
                let buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("meshlet_visibility_buffer_draw_index_buffer"),
                    size: needed_buffer_size,
                    usage: BufferUsages::STORAGE | BufferUsages::INDEX,
                    mapped_at_creation: false,
                });
                *slot = Some(buffer.clone());
                buffer
            }
        };

    let needed_buffer_size = gpu_scene.scene_meshlet_count.div_ceil(32) as u64 * 4;
    for (view_entity, view, render_layers, (_, shadow_view)) in &views {
        let instance_visibility = gpu_scene
            .view_instance_visibility
            .entry(view_entity)
            .or_insert_with(|| {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_view_instance_visibility"));
                buffer
            });
        for (instance_index, (_, layers, not_shadow_caster)) in
            gpu_scene.instances.iter().enumerate()
        {
            // If either the layers don't match the view's layers or this is a shadow view
            // and the instance is not a shadow caster, hide the instance for this view
            if !render_layers.unwrap_or(&default()).intersects(layers)
                || (shadow_view.is_some() && *not_shadow_caster)
            {
                let vec = instance_visibility.get_mut();
                let index = instance_index / 32;
                let bit = instance_index - index * 32;
                if vec.len() <= index {
                    vec.extend(iter::repeat(0).take(index - vec.len() + 1));
                }
                vec[index] |= 1 << bit;
            }
        }
        upload_storage_buffer(instance_visibility, &render_device, &render_queue);
        let instance_visibility = instance_visibility.buffer().unwrap().clone();

        let create_occlusion_buffer = || {
            render_device.create_buffer(&BufferDescriptor {
                label: Some("meshlet_occlusion_buffer"),
                size: needed_buffer_size,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let (previous_occlusion_buffer, occlusion_buffer, occlusion_buffer_needs_clearing) =
            match gpu_scene.previous_occlusion_buffers.get(&view_entity) {
                Some((buffer_a, buffer_b)) if buffer_b.size() >= needed_buffer_size => {
                    (buffer_a.clone(), buffer_b.clone(), true)
                }
                Some((buffer_a, _)) => (buffer_a.clone(), create_occlusion_buffer(), false),
                None => (create_occlusion_buffer(), create_occlusion_buffer(), false),
            };
        gpu_scene.previous_occlusion_buffers.insert(
            view_entity,
            (occlusion_buffer.clone(), previous_occlusion_buffer.clone()),
        );

        let visibility_buffer = TextureDescriptor {
            label: Some("meshlet_visibility_buffer"),
            size: Extent3d {
                width: view.viewport.z,
                height: view.viewport.w,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Uint,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let visibility_buffer_draw_indirect_args_first =
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("meshlet_visibility_buffer_draw_indirect_args_first"),
                contents: DrawIndirectArgs {
                    vertex_count: 0,
                    instance_count: 1,
                    first_vertex: 0,
                    first_instance: 0,
                }
                .as_bytes(),
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            });
        let visibility_buffer_draw_indirect_args_second =
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("meshlet_visibility_buffer_draw_indirect_args_second"),
                contents: DrawIndirectArgs {
                    vertex_count: 0,
                    instance_count: 1,
                    first_vertex: 0,
                    first_instance: 0,
                }
                .as_bytes(),
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            });

        let depth_size = Extent3d {
            // If not a power of 2, round down to the nearest power of 2 to ensure depth is conservative
            width: 1 << (31 - view.viewport.z.leading_zeros()),
            height: 1 << (31 - view.viewport.w.leading_zeros()),
            depth_or_array_layers: 1,
        };
        let depth_mip_count = depth_size.width.max(depth_size.height).ilog2() + 1;
        let depth_pyramid = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("meshlet_depth_pyramid"),
                size: depth_size,
                mip_level_count: depth_mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let depth_pyramid_mips = (0..depth_mip_count)
            .map(|i| {
                depth_pyramid.texture.create_view(&TextureViewDescriptor {
                    label: Some("meshlet_depth_pyramid_texture_view"),
                    format: Some(TextureFormat::R32Float),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: i,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                })
            })
            .collect();

        let material_depth_color = TextureDescriptor {
            label: Some("meshlet_material_depth_color"),
            size: Extent3d {
                width: view.viewport.z,
                height: view.viewport.w,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let material_depth = TextureDescriptor {
            label: Some("meshlet_material_depth"),
            size: Extent3d {
                width: view.viewport.z,
                height: view.viewport.w,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth16Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let not_shadow_view = shadow_view.is_none();
        commands.entity(view_entity).insert(MeshletViewResources {
            scene_meshlet_count: gpu_scene.scene_meshlet_count,
            previous_occlusion_buffer,
            occlusion_buffer,
            occlusion_buffer_needs_clearing,
            instance_visibility,
            visibility_buffer: not_shadow_view
                .then(|| texture_cache.get(&render_device, visibility_buffer)),
            visibility_buffer_draw_indirect_args_first,
            visibility_buffer_draw_indirect_args_second,
            visibility_buffer_draw_index_buffer: visibility_buffer_draw_index_buffer.clone(),
            depth_pyramid,
            depth_pyramid_mips,
            material_depth_color: not_shadow_view
                .then(|| texture_cache.get(&render_device, material_depth_color)),
            material_depth: not_shadow_view
                .then(|| texture_cache.get(&render_device, material_depth)),
        });
    }
}

pub fn prepare_meshlet_view_bind_groups(
    gpu_scene: Res<MeshletGpuScene>,
    views: Query<(
        Entity,
        &MeshletViewResources,
        AnyOf<(&ViewDepthTexture, &ShadowView)>,
    )>,
    view_uniforms: Res<ViewUniforms>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    let Some(view_uniforms) = view_uniforms.uniforms.binding() else {
        return;
    };

    for (view_entity, view_resources, view_depth) in &views {
        let entries = BindGroupEntries::sequential((
            gpu_scene.thread_meshlet_ids.binding().unwrap(),
            gpu_scene.meshlet_bounding_spheres.binding(),
            gpu_scene.thread_instance_ids.binding().unwrap(),
            gpu_scene.instance_uniforms.binding().unwrap(),
            gpu_scene.view_instance_visibility[&view_entity]
                .binding()
                .unwrap(),
            view_resources.occlusion_buffer.as_entire_binding(),
            gpu_scene.previous_cluster_ids.binding().unwrap(),
            view_resources.previous_occlusion_buffer.as_entire_binding(),
            view_uniforms.clone(),
            &view_resources.depth_pyramid.default_view,
        ));
        let culling = render_device.create_bind_group(
            "meshlet_culling_bind_group",
            &gpu_scene.culling_bind_group_layout,
            &entries,
        );

        let entries = BindGroupEntries::sequential((
            view_resources.occlusion_buffer.as_entire_binding(),
            gpu_scene.thread_meshlet_ids.binding().unwrap(),
            gpu_scene.previous_cluster_ids.binding().unwrap(),
            view_resources.previous_occlusion_buffer.as_entire_binding(),
            gpu_scene.meshlets.binding(),
            view_resources
                .visibility_buffer_draw_indirect_args_first
                .as_entire_binding(),
            view_resources
                .visibility_buffer_draw_index_buffer
                .as_entire_binding(),
        ));
        let write_index_buffer_first = render_device.create_bind_group(
            "meshlet_write_index_buffer_first_bind_group",
            &gpu_scene.write_index_buffer_bind_group_layout,
            &entries,
        );

        let entries = BindGroupEntries::sequential((
            view_resources.occlusion_buffer.as_entire_binding(),
            gpu_scene.thread_meshlet_ids.binding().unwrap(),
            gpu_scene.previous_cluster_ids.binding().unwrap(),
            view_resources.previous_occlusion_buffer.as_entire_binding(),
            gpu_scene.meshlets.binding(),
            view_resources
                .visibility_buffer_draw_indirect_args_second
                .as_entire_binding(),
            view_resources
                .visibility_buffer_draw_index_buffer
                .as_entire_binding(),
        ));
        let write_index_buffer_second = render_device.create_bind_group(
            "meshlet_write_index_buffer_second_bind_group",
            &gpu_scene.write_index_buffer_bind_group_layout,
            &entries,
        );

        let view_depth_texture = match view_depth {
            (Some(view_depth), None) => view_depth.view(),
            (None, Some(shadow_view)) => &shadow_view.depth_attachment.view,
            _ => unreachable!(),
        };
        let downsample_depth = (0..view_resources.depth_pyramid_mips.len())
            .map(|i| {
                render_device.create_bind_group(
                    "meshlet_downsample_depth_bind_group",
                    &gpu_scene.downsample_depth_bind_group_layout,
                    &BindGroupEntries::sequential((
                        if i == 0 {
                            view_depth_texture
                        } else {
                            &view_resources.depth_pyramid_mips[i - 1]
                        },
                        &gpu_scene.depth_pyramid_sampler,
                    )),
                )
            })
            .collect();

        let entries = BindGroupEntries::sequential((
            gpu_scene.thread_meshlet_ids.binding().unwrap(),
            gpu_scene.meshlets.binding(),
            gpu_scene.indices.binding(),
            gpu_scene.vertex_ids.binding(),
            gpu_scene.vertex_data.binding(),
            gpu_scene.thread_instance_ids.binding().unwrap(),
            gpu_scene.instance_uniforms.binding().unwrap(),
            gpu_scene.instance_material_ids.binding().unwrap(),
            view_resources
                .visibility_buffer_draw_index_buffer
                .as_entire_binding(),
            view_uniforms.clone(),
        ));
        let visibility_buffer_raster = render_device.create_bind_group(
            "meshlet_visibility_raster_buffer_bind_group",
            &gpu_scene.visibility_buffer_raster_bind_group_layout,
            &entries,
        );

        let copy_material_depth =
            view_resources
                .material_depth_color
                .as_ref()
                .map(|material_depth_color| {
                    render_device.create_bind_group(
                        "meshlet_copy_material_depth_bind_group",
                        &gpu_scene.copy_material_depth_bind_group_layout,
                        &[BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(
                                &material_depth_color.default_view,
                            ),
                        }],
                    )
                });

        let material_draw = view_resources
            .visibility_buffer
            .as_ref()
            .map(|visibility_buffer| {
                let entries = BindGroupEntries::sequential((
                    &visibility_buffer.default_view,
                    gpu_scene.thread_meshlet_ids.binding().unwrap(),
                    gpu_scene.meshlets.binding(),
                    gpu_scene.indices.binding(),
                    gpu_scene.vertex_ids.binding(),
                    gpu_scene.vertex_data.binding(),
                    gpu_scene.thread_instance_ids.binding().unwrap(),
                    gpu_scene.instance_uniforms.binding().unwrap(),
                ));
                render_device.create_bind_group(
                    "meshlet_mesh_material_draw_bind_group",
                    &gpu_scene.material_draw_bind_group_layout,
                    &entries,
                )
            });

        commands.entity(view_entity).insert(MeshletViewBindGroups {
            culling,
            write_index_buffer_first,
            write_index_buffer_second,
            downsample_depth,
            visibility_buffer_raster,
            copy_material_depth,
            material_draw,
        });
    }
}

/// A resource that manages GPU data for rendering [`MeshletMesh`]'s.
#[derive(Resource)]
pub struct MeshletGpuScene {
    vertex_data: PersistentGpuBuffer<Arc<[u8]>>,
    vertex_ids: PersistentGpuBuffer<Arc<[u32]>>,
    indices: PersistentGpuBuffer<Arc<[u8]>>,
    meshlets: PersistentGpuBuffer<Arc<[Meshlet]>>,
    meshlet_bounding_spheres: PersistentGpuBuffer<Arc<[MeshletBoundingSphere]>>,
    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, ([Range<BufferAddress>; 5], u64)>,

    scene_meshlet_count: u32,
    scene_triangle_count: u64,
    next_material_id: u32,
    material_id_lookup: HashMap<UntypedAssetId, u32>,
    material_ids_present_in_scene: HashSet<u32>,
    /// Per-instance Entity, RenderLayers, and NotShadowCaster
    instances: Vec<(Entity, RenderLayers, bool)>,
    /// Per-instance transforms, model matrices, and render flags
    instance_uniforms: StorageBuffer<Vec<MeshUniform>>,
    /// Per-view per-instance visibility bit. Used for RenderLayer and NotShadowCaster support.
    view_instance_visibility: EntityHashMap<StorageBuffer<Vec<u32>>>,
    instance_material_ids: StorageBuffer<Vec<u32>>,
    thread_instance_ids: StorageBuffer<Vec<u32>>,
    thread_meshlet_ids: StorageBuffer<Vec<u32>>,
    previous_cluster_ids: StorageBuffer<Vec<u32>>,
    previous_cluster_id_starts: HashMap<(Entity, AssetId<MeshletMesh>), (u32, bool)>,
    previous_occlusion_buffers: EntityHashMap<(Buffer, Buffer)>,
    visibility_buffer_draw_index_buffer: Option<Buffer>,

    culling_bind_group_layout: BindGroupLayout,
    write_index_buffer_bind_group_layout: BindGroupLayout,
    visibility_buffer_raster_bind_group_layout: BindGroupLayout,
    downsample_depth_bind_group_layout: BindGroupLayout,
    copy_material_depth_bind_group_layout: BindGroupLayout,
    material_draw_bind_group_layout: BindGroupLayout,
    depth_pyramid_sampler: Sampler,
}

impl FromWorld for MeshletGpuScene {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        Self {
            vertex_data: PersistentGpuBuffer::new("meshlet_vertex_data", render_device),
            vertex_ids: PersistentGpuBuffer::new("meshlet_vertex_ids", render_device),
            indices: PersistentGpuBuffer::new("meshlet_indices", render_device),
            meshlets: PersistentGpuBuffer::new("meshlets", render_device),
            meshlet_bounding_spheres: PersistentGpuBuffer::new(
                "meshlet_bounding_spheres",
                render_device,
            ),
            meshlet_mesh_slices: HashMap::new(),

            scene_meshlet_count: 0,
            scene_triangle_count: 0,
            next_material_id: 0,
            material_id_lookup: HashMap::new(),
            material_ids_present_in_scene: HashSet::new(),
            instances: Vec::new(),
            instance_uniforms: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_uniforms"));
                buffer
            },
            view_instance_visibility: EntityHashMap::default(),
            instance_material_ids: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_material_ids"));
                buffer
            },
            thread_instance_ids: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_thread_instance_ids"));
                buffer
            },
            thread_meshlet_ids: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_thread_meshlet_ids"));
                buffer
            },
            previous_cluster_ids: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_previous_cluster_ids"));
                buffer
            },
            previous_cluster_id_starts: HashMap::new(),
            previous_occlusion_buffers: EntityHashMap::default(),
            visibility_buffer_draw_index_buffer: None,

            // TODO: Buffer min sizes
            culling_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_culling_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::COMPUTE,
                    (
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        uniform_buffer::<ViewUniform>(true),
                        texture_2d(TextureSampleType::Float { filterable: false }),
                    ),
                ),
            ),
            write_index_buffer_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_write_index_buffer_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::COMPUTE,
                    (
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_sized(false, None),
                        storage_buffer_sized(false, None),
                    ),
                ),
            ),
            downsample_depth_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_downsample_depth_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        sampler(SamplerBindingType::NonFiltering),
                    ),
                ),
            ),
            visibility_buffer_raster_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_visibility_buffer_raster_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX,
                    (
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        uniform_buffer::<ViewUniform>(true),
                    ),
                ),
            ),
            copy_material_depth_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_copy_material_depth_bind_group_layout",
                &BindGroupLayoutEntries::single(
                    ShaderStages::FRAGMENT,
                    texture_2d(TextureSampleType::Uint),
                ),
            ),
            material_draw_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_mesh_material_draw_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Uint),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                    ),
                ),
            ),
            depth_pyramid_sampler: render_device.create_sampler(&SamplerDescriptor {
                label: Some("meshlet_depth_pyramid_sampler"),
                ..default()
            }),
        }
    }
}

impl MeshletGpuScene {
    /// Clear per-frame CPU->GPU upload buffers and reset all per-frame data.
    fn reset(&mut self) {
        // TODO: Shrink capacity if saturation is low
        self.scene_meshlet_count = 0;
        self.scene_triangle_count = 0;
        self.next_material_id = 0;
        self.material_id_lookup.clear();
        self.material_ids_present_in_scene.clear();
        self.instances.clear();
        self.view_instance_visibility
            .values_mut()
            .for_each(|b| b.get_mut().clear());
        self.instance_uniforms.get_mut().clear();
        self.instance_material_ids.get_mut().clear();
        self.thread_instance_ids.get_mut().clear();
        self.thread_meshlet_ids.get_mut().clear();
        self.previous_cluster_ids.get_mut().clear();
        self.previous_cluster_id_starts
            .values_mut()
            .for_each(|(_, active)| *active = false);
        // TODO: Remove unused entries for previous_occlusion_buffers
    }

    fn queue_meshlet_mesh_upload(
        &mut self,
        instance: Entity,
        render_layers: RenderLayers,
        not_shadow_caster: bool,
        handle: &Handle<MeshletMesh>,
        assets: &mut Assets<MeshletMesh>,
        instance_index: u32,
    ) {
        let queue_meshlet_mesh = |asset_id: &AssetId<MeshletMesh>| {
            let meshlet_mesh = assets.remove_untracked(*asset_id).expect(
                "MeshletMesh asset was already unloaded but is not registered with MeshletGpuScene",
            );

            let vertex_data_slice = self
                .vertex_data
                .queue_write(Arc::clone(&meshlet_mesh.vertex_data), ());
            let vertex_ids_slice = self.vertex_ids.queue_write(
                Arc::clone(&meshlet_mesh.vertex_ids),
                vertex_data_slice.start,
            );
            let indices_slice = self
                .indices
                .queue_write(Arc::clone(&meshlet_mesh.indices), ());
            let meshlets_slice = self.meshlets.queue_write(
                Arc::clone(&meshlet_mesh.meshlets),
                (vertex_ids_slice.start, indices_slice.start),
            );
            let meshlet_bounding_spheres_slice = self
                .meshlet_bounding_spheres
                .queue_write(Arc::clone(&meshlet_mesh.meshlet_bounding_spheres), ());

            (
                [
                    vertex_data_slice,
                    vertex_ids_slice,
                    indices_slice,
                    meshlets_slice,
                    meshlet_bounding_spheres_slice,
                ],
                meshlet_mesh.total_meshlet_triangles,
            )
        };

        // Append instance data for this frame
        self.instances
            .push((instance, render_layers, not_shadow_caster));
        self.instance_material_ids.get_mut().push(0);

        // If the MeshletMesh asset has not been uploaded to the GPU yet, queue it for uploading
        let ([_, _, _, meshlets_slice, _], triangle_count) = self
            .meshlet_mesh_slices
            .entry(handle.id())
            .or_insert_with_key(queue_meshlet_mesh)
            .clone();

        let meshlets_slice = (meshlets_slice.start as u32 / size_of::<Meshlet>() as u32)
            ..(meshlets_slice.end as u32 / size_of::<Meshlet>() as u32);

        let current_cluster_id_start = self.scene_meshlet_count;

        self.scene_meshlet_count += meshlets_slice.end - meshlets_slice.start;
        self.scene_triangle_count += triangle_count;

        // Calculate the previous cluster IDs for each meshlet for this instance
        let previous_cluster_id_start = self
            .previous_cluster_id_starts
            .entry((instance, handle.id()))
            .or_insert((0, true));
        let previous_cluster_ids = if previous_cluster_id_start.1 {
            0..(meshlets_slice.len() as u32)
        } else {
            let start = previous_cluster_id_start.0;
            start..(meshlets_slice.len() as u32 + start)
        };

        // Append per-cluster data for this frame
        self.thread_instance_ids
            .get_mut()
            .extend(std::iter::repeat(instance_index).take(meshlets_slice.len()));
        self.thread_meshlet_ids.get_mut().extend(meshlets_slice);
        self.previous_cluster_ids
            .get_mut()
            .extend(previous_cluster_ids);

        *previous_cluster_id_start = (current_cluster_id_start, true);
    }

    /// Get the depth value for use with the material depth texture for a given [`Material`] asset.
    pub fn get_material_id(&mut self, material_id: UntypedAssetId) -> u32 {
        *self
            .material_id_lookup
            .entry(material_id)
            .or_insert_with(|| {
                self.next_material_id += 1;
                self.next_material_id
            })
    }

    pub fn material_present_in_scene(&self, material_id: &u32) -> bool {
        self.material_ids_present_in_scene.contains(material_id)
    }

    pub fn culling_bind_group_layout(&self) -> BindGroupLayout {
        self.culling_bind_group_layout.clone()
    }

    pub fn write_index_buffer_bind_group_layout(&self) -> BindGroupLayout {
        self.write_index_buffer_bind_group_layout.clone()
    }

    pub fn downsample_depth_bind_group_layout(&self) -> BindGroupLayout {
        self.downsample_depth_bind_group_layout.clone()
    }

    pub fn visibility_buffer_raster_bind_group_layout(&self) -> BindGroupLayout {
        self.visibility_buffer_raster_bind_group_layout.clone()
    }

    pub fn copy_material_depth_bind_group_layout(&self) -> BindGroupLayout {
        self.copy_material_depth_bind_group_layout.clone()
    }

    pub fn material_draw_bind_group_layout(&self) -> BindGroupLayout {
        self.material_draw_bind_group_layout.clone()
    }
}

#[derive(Component)]
pub struct MeshletViewResources {
    pub scene_meshlet_count: u32,
    previous_occlusion_buffer: Buffer,
    pub occlusion_buffer: Buffer,
    pub occlusion_buffer_needs_clearing: bool,
    pub instance_visibility: Buffer,
    pub visibility_buffer: Option<CachedTexture>,
    pub visibility_buffer_draw_indirect_args_first: Buffer,
    pub visibility_buffer_draw_indirect_args_second: Buffer,
    visibility_buffer_draw_index_buffer: Buffer,
    pub depth_pyramid: CachedTexture,
    pub depth_pyramid_mips: Box<[TextureView]>,
    pub material_depth_color: Option<CachedTexture>,
    pub material_depth: Option<CachedTexture>,
}

#[derive(Component)]
pub struct MeshletViewBindGroups {
    pub culling: BindGroup,
    pub write_index_buffer_first: BindGroup,
    pub write_index_buffer_second: BindGroup,
    pub downsample_depth: Box<[BindGroup]>,
    pub visibility_buffer_raster: BindGroup,
    pub copy_material_depth: Option<BindGroup>,
    pub material_draw: Option<BindGroup>,
}
