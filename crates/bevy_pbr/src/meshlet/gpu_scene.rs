use super::{
    asset::{Meshlet, MeshletBoundingSpheres, MeshletMesh},
    persistent_buffer::PersistentGpuBuffer,
};
use crate::{
    Material, MeshFlags, MeshTransforms, MeshUniform, NotShadowCaster, NotShadowReceiver,
    PreviousGlobalTransform, RenderMaterialInstances, ShadowView,
};
use bevy_asset::{AssetEvent, AssetId, AssetServer, Assets, Handle, UntypedAssetId};
use bevy_core_pipeline::{
    core_3d::Camera3d,
    prepass::{PreviousViewData, PreviousViewUniforms},
};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    event::EventReader,
    query::{AnyOf, Has},
    system::{Commands, Local, Query, Res, ResMut, Resource, SystemState},
    world::{FromWorld, World},
};
use bevy_math::{UVec2, Vec4Swizzles};
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
    array, iter,
    mem::size_of,
    ops::{DerefMut, Range},
    sync::{atomic::AtomicBool, Arc},
};

/// Create and queue for uploading to the GPU [`MeshUniform`] components for
/// [`MeshletMesh`] entities, as well as queuing uploads for any new meshlet mesh
/// assets that have not already been uploaded to the GPU.
pub fn extract_meshlet_meshes(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    // TODO: Replace main_world and system_state when Extract<ResMut<Assets<MeshletMesh>>> is possible
    mut main_world: ResMut<MainWorld>,
    mut system_state: Local<
        Option<
            SystemState<(
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
            )>,
        >,
    >,
) {
    if system_state.is_none() {
        *system_state = Some(SystemState::new(&mut main_world));
    }
    let system_state = system_state.as_mut().unwrap();

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
        instance,
        handle,
        transform,
        previous_transform,
        render_layers,
        not_shadow_receiver,
        not_shadow_caster,
    ) in &instances_query
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
            world_from_local: (&transform).into(),
            previous_world_from_local: (&previous_transform).into(),
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
fn upload_storage_buffer<T: ShaderSize + bytemuck::NoUninit>(
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
        let bytes = bytemuck::must_cast_slice(buffer.get().as_slice());
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
        &mut gpu_scene.instance_meshlet_counts_prefix_sum,
        &render_device,
        &render_queue,
    );
    upload_storage_buffer(
        &mut gpu_scene.instance_meshlet_slice_starts,
        &render_device,
        &render_queue,
    );

    // Early submission for GPU data uploads to start while the render graph records commands
    render_queue.submit([]);

    let needed_buffer_size = 4 * gpu_scene.scene_meshlet_count as u64;
    match &mut gpu_scene.cluster_instance_ids {
        Some(buffer) if buffer.size() >= needed_buffer_size => buffer.clone(),
        slot => {
            let buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("meshlet_cluster_instance_ids"),
                size: needed_buffer_size,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            *slot = Some(buffer.clone());
            buffer
        }
    };
    match &mut gpu_scene.cluster_meshlet_ids {
        Some(buffer) if buffer.size() >= needed_buffer_size => buffer.clone(),
        slot => {
            let buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("meshlet_cluster_meshlet_ids"),
                size: needed_buffer_size,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            *slot = Some(buffer.clone());
            buffer
        }
    };

    let needed_buffer_size = 4 * gpu_scene.scene_triangle_count;
    let visibility_buffer_draw_triangle_buffer =
        match &mut gpu_scene.visibility_buffer_draw_triangle_buffer {
            Some(buffer) if buffer.size() >= needed_buffer_size => buffer.clone(),
            slot => {
                let buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("meshlet_visibility_buffer_draw_triangle_buffer"),
                    size: needed_buffer_size,
                    usage: BufferUsages::STORAGE,
                    mapped_at_creation: false,
                });
                *slot = Some(buffer.clone());
                buffer
            }
        };

    let needed_buffer_size =
        gpu_scene.scene_meshlet_count.div_ceil(u32::BITS) as u64 * size_of::<u32>() as u64;
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

        let second_pass_candidates_buffer = match &mut gpu_scene.second_pass_candidates_buffer {
            Some(buffer) if buffer.size() >= needed_buffer_size => buffer.clone(),
            slot => {
                let buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("meshlet_second_pass_candidates"),
                    size: needed_buffer_size,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                *slot = Some(buffer.clone());
                buffer
            }
        };

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

        let depth_pyramid_size = Extent3d {
            width: view.viewport.z.div_ceil(2),
            height: view.viewport.w.div_ceil(2),
            depth_or_array_layers: 1,
        };
        let depth_pyramid_mip_count = depth_pyramid_size.max_mips(TextureDimension::D2);
        let depth_pyramid = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("meshlet_depth_pyramid"),
                size: depth_pyramid_size,
                mip_level_count: depth_pyramid_mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let depth_pyramid_mips = array::from_fn(|i| {
            if (i as u32) < depth_pyramid_mip_count {
                depth_pyramid.texture.create_view(&TextureViewDescriptor {
                    label: Some("meshlet_depth_pyramid_texture_view"),
                    format: Some(TextureFormat::R32Float),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: i as u32,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                })
            } else {
                gpu_scene.depth_pyramid_dummy_texture.clone()
            }
        });
        let depth_pyramid_all_mips = depth_pyramid.default_view.clone();

        let previous_depth_pyramid = match gpu_scene.previous_depth_pyramids.get(&view_entity) {
            Some(texture_view) => texture_view.clone(),
            None => depth_pyramid_all_mips.clone(),
        };
        gpu_scene
            .previous_depth_pyramids
            .insert(view_entity, depth_pyramid_all_mips.clone());

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
            second_pass_candidates_buffer,
            instance_visibility,
            visibility_buffer: not_shadow_view
                .then(|| texture_cache.get(&render_device, visibility_buffer)),
            visibility_buffer_draw_indirect_args_first,
            visibility_buffer_draw_indirect_args_second,
            visibility_buffer_draw_triangle_buffer: visibility_buffer_draw_triangle_buffer.clone(),
            depth_pyramid_all_mips,
            depth_pyramid_mips,
            depth_pyramid_mip_count,
            previous_depth_pyramid,
            material_depth_color: not_shadow_view
                .then(|| texture_cache.get(&render_device, material_depth_color)),
            material_depth: not_shadow_view
                .then(|| texture_cache.get(&render_device, material_depth)),
            view_size: view.viewport.zw(),
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
    previous_view_uniforms: Res<PreviousViewUniforms>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    let (
        Some(cluster_instance_ids),
        Some(cluster_meshlet_ids),
        Some(view_uniforms),
        Some(previous_view_uniforms),
    ) = (
        gpu_scene.cluster_instance_ids.as_ref(),
        gpu_scene.cluster_meshlet_ids.as_ref(),
        view_uniforms.uniforms.binding(),
        previous_view_uniforms.uniforms.binding(),
    )
    else {
        return;
    };

    let first_node = Arc::new(AtomicBool::new(true));

    // TODO: Some of these bind groups can be reused across multiple views
    for (view_entity, view_resources, view_depth) in &views {
        let entries = BindGroupEntries::sequential((
            gpu_scene
                .instance_meshlet_counts_prefix_sum
                .binding()
                .unwrap(),
            gpu_scene.instance_meshlet_slice_starts.binding().unwrap(),
            cluster_instance_ids.as_entire_binding(),
            cluster_meshlet_ids.as_entire_binding(),
        ));
        let fill_cluster_buffers = render_device.create_bind_group(
            "meshlet_fill_cluster_buffers",
            &gpu_scene.fill_cluster_buffers_bind_group_layout,
            &entries,
        );

        let entries = BindGroupEntries::sequential((
            cluster_meshlet_ids.as_entire_binding(),
            gpu_scene.meshlet_bounding_spheres.binding(),
            cluster_instance_ids.as_entire_binding(),
            gpu_scene.instance_uniforms.binding().unwrap(),
            view_resources.instance_visibility.as_entire_binding(),
            view_resources
                .second_pass_candidates_buffer
                .as_entire_binding(),
            gpu_scene.meshlets.binding(),
            view_resources
                .visibility_buffer_draw_indirect_args_first
                .as_entire_binding(),
            view_resources
                .visibility_buffer_draw_triangle_buffer
                .as_entire_binding(),
            &view_resources.previous_depth_pyramid,
            view_uniforms.clone(),
            previous_view_uniforms.clone(),
        ));
        let culling_first = render_device.create_bind_group(
            "meshlet_culling_first_bind_group",
            &gpu_scene.culling_bind_group_layout,
            &entries,
        );

        let entries = BindGroupEntries::sequential((
            cluster_meshlet_ids.as_entire_binding(),
            gpu_scene.meshlet_bounding_spheres.binding(),
            cluster_instance_ids.as_entire_binding(),
            gpu_scene.instance_uniforms.binding().unwrap(),
            view_resources.instance_visibility.as_entire_binding(),
            view_resources
                .second_pass_candidates_buffer
                .as_entire_binding(),
            gpu_scene.meshlets.binding(),
            view_resources
                .visibility_buffer_draw_indirect_args_second
                .as_entire_binding(),
            view_resources
                .visibility_buffer_draw_triangle_buffer
                .as_entire_binding(),
            &view_resources.depth_pyramid_all_mips,
            view_uniforms.clone(),
            previous_view_uniforms.clone(),
        ));
        let culling_second = render_device.create_bind_group(
            "meshlet_culling_second_bind_group",
            &gpu_scene.culling_bind_group_layout,
            &entries,
        );

        let view_depth_texture = match view_depth {
            (Some(view_depth), None) => view_depth.view(),
            (None, Some(shadow_view)) => &shadow_view.depth_attachment.view,
            _ => unreachable!(),
        };
        let downsample_depth = render_device.create_bind_group(
            "meshlet_downsample_depth_bind_group",
            &gpu_scene.downsample_depth_bind_group_layout,
            &BindGroupEntries::sequential((
                view_depth_texture,
                &view_resources.depth_pyramid_mips[0],
                &view_resources.depth_pyramid_mips[1],
                &view_resources.depth_pyramid_mips[2],
                &view_resources.depth_pyramid_mips[3],
                &view_resources.depth_pyramid_mips[4],
                &view_resources.depth_pyramid_mips[5],
                &view_resources.depth_pyramid_mips[6],
                &view_resources.depth_pyramid_mips[7],
                &view_resources.depth_pyramid_mips[8],
                &view_resources.depth_pyramid_mips[9],
                &view_resources.depth_pyramid_mips[10],
                &view_resources.depth_pyramid_mips[11],
                &gpu_scene.depth_pyramid_sampler,
            )),
        );

        let entries = BindGroupEntries::sequential((
            cluster_meshlet_ids.as_entire_binding(),
            gpu_scene.meshlets.binding(),
            gpu_scene.indices.binding(),
            gpu_scene.vertex_ids.binding(),
            gpu_scene.vertex_data.binding(),
            cluster_instance_ids.as_entire_binding(),
            gpu_scene.instance_uniforms.binding().unwrap(),
            gpu_scene.instance_material_ids.binding().unwrap(),
            view_resources
                .visibility_buffer_draw_triangle_buffer
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
                    cluster_meshlet_ids.as_entire_binding(),
                    gpu_scene.meshlets.binding(),
                    gpu_scene.indices.binding(),
                    gpu_scene.vertex_ids.binding(),
                    gpu_scene.vertex_data.binding(),
                    cluster_instance_ids.as_entire_binding(),
                    gpu_scene.instance_uniforms.binding().unwrap(),
                ));
                render_device.create_bind_group(
                    "meshlet_mesh_material_draw_bind_group",
                    &gpu_scene.material_draw_bind_group_layout,
                    &entries,
                )
            });

        commands.entity(view_entity).insert(MeshletViewBindGroups {
            first_node: Arc::clone(&first_node),
            fill_cluster_buffers,
            culling_first,
            culling_second,
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
    meshlet_bounding_spheres: PersistentGpuBuffer<Arc<[MeshletBoundingSpheres]>>,
    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, ([Range<BufferAddress>; 5], u64)>,

    scene_meshlet_count: u32,
    scene_triangle_count: u64,
    next_material_id: u32,
    material_id_lookup: HashMap<UntypedAssetId, u32>,
    material_ids_present_in_scene: HashSet<u32>,
    /// Per-instance [`Entity`], [`RenderLayers`], and [`NotShadowCaster`]
    instances: Vec<(Entity, RenderLayers, bool)>,
    /// Per-instance transforms, model matrices, and render flags
    instance_uniforms: StorageBuffer<Vec<MeshUniform>>,
    /// Per-view per-instance visibility bit. Used for [`RenderLayers`] and [`NotShadowCaster`] support.
    view_instance_visibility: EntityHashMap<StorageBuffer<Vec<u32>>>,
    instance_material_ids: StorageBuffer<Vec<u32>>,
    instance_meshlet_counts_prefix_sum: StorageBuffer<Vec<u32>>,
    instance_meshlet_slice_starts: StorageBuffer<Vec<u32>>,
    cluster_instance_ids: Option<Buffer>,
    cluster_meshlet_ids: Option<Buffer>,
    second_pass_candidates_buffer: Option<Buffer>,
    previous_depth_pyramids: EntityHashMap<TextureView>,
    visibility_buffer_draw_triangle_buffer: Option<Buffer>,

    fill_cluster_buffers_bind_group_layout: BindGroupLayout,
    culling_bind_group_layout: BindGroupLayout,
    visibility_buffer_raster_bind_group_layout: BindGroupLayout,
    downsample_depth_bind_group_layout: BindGroupLayout,
    copy_material_depth_bind_group_layout: BindGroupLayout,
    material_draw_bind_group_layout: BindGroupLayout,
    depth_pyramid_sampler: Sampler,
    depth_pyramid_dummy_texture: TextureView,
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
            instance_meshlet_counts_prefix_sum: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_meshlet_counts_prefix_sum"));
                buffer
            },
            instance_meshlet_slice_starts: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_meshlet_slice_starts"));
                buffer
            },
            cluster_instance_ids: None,
            cluster_meshlet_ids: None,
            second_pass_candidates_buffer: None,
            previous_depth_pyramids: EntityHashMap::default(),
            visibility_buffer_draw_triangle_buffer: None,

            // TODO: Buffer min sizes
            fill_cluster_buffers_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_fill_cluster_buffers_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::COMPUTE,
                    (
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_sized(false, None),
                        storage_buffer_sized(false, None),
                    ),
                ),
            ),
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
                        storage_buffer_sized(false, None),
                        storage_buffer_sized(false, None),
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        uniform_buffer::<ViewUniform>(true),
                        uniform_buffer::<PreviousViewData>(true),
                    ),
                ),
            ),
            downsample_depth_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_downsample_depth_bind_group_layout",
                &BindGroupLayoutEntries::sequential(ShaderStages::COMPUTE, {
                    let write_only_r32float = || {
                        texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly)
                    };
                    (
                        texture_depth_2d(),
                        write_only_r32float(),
                        write_only_r32float(),
                        write_only_r32float(),
                        write_only_r32float(),
                        write_only_r32float(),
                        texture_storage_2d(
                            TextureFormat::R32Float,
                            StorageTextureAccess::ReadWrite,
                        ),
                        write_only_r32float(),
                        write_only_r32float(),
                        write_only_r32float(),
                        write_only_r32float(),
                        write_only_r32float(),
                        write_only_r32float(),
                        sampler(SamplerBindingType::NonFiltering),
                    )
                }),
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
            depth_pyramid_dummy_texture: render_device
                .create_texture(&TextureDescriptor {
                    label: Some("meshlet_depth_pyramid_dummy_texture"),
                    size: Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::R32Float,
                    usage: TextureUsages::STORAGE_BINDING,
                    view_formats: &[],
                })
                .create_view(&TextureViewDescriptor {
                    label: Some("meshlet_depth_pyramid_dummy_texture_view"),
                    format: Some(TextureFormat::R32Float),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
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
        self.instance_meshlet_counts_prefix_sum.get_mut().clear();
        self.instance_meshlet_slice_starts.get_mut().clear();
        // TODO: Remove unused entries for view_instance_visibility and previous_depth_pyramids
    }

    fn queue_meshlet_mesh_upload(
        &mut self,
        instance: Entity,
        render_layers: RenderLayers,
        not_shadow_caster: bool,
        handle: &Handle<MeshletMesh>,
        assets: &mut Assets<MeshletMesh>,
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
                .queue_write(Arc::clone(&meshlet_mesh.bounding_spheres), ());

            (
                [
                    vertex_data_slice,
                    vertex_ids_slice,
                    indices_slice,
                    meshlets_slice,
                    meshlet_bounding_spheres_slice,
                ],
                meshlet_mesh.worst_case_meshlet_triangles,
            )
        };

        // If the MeshletMesh asset has not been uploaded to the GPU yet, queue it for uploading
        let ([_, _, _, meshlets_slice, _], triangle_count) = self
            .meshlet_mesh_slices
            .entry(handle.id())
            .or_insert_with_key(queue_meshlet_mesh)
            .clone();

        let meshlets_slice = (meshlets_slice.start as u32 / size_of::<Meshlet>() as u32)
            ..(meshlets_slice.end as u32 / size_of::<Meshlet>() as u32);

        // Append instance data for this frame
        self.instances
            .push((instance, render_layers, not_shadow_caster));
        self.instance_material_ids.get_mut().push(0);
        self.instance_meshlet_counts_prefix_sum
            .get_mut()
            .push(self.scene_meshlet_count);
        self.instance_meshlet_slice_starts
            .get_mut()
            .push(meshlets_slice.start);

        self.scene_meshlet_count += meshlets_slice.end - meshlets_slice.start;
        self.scene_triangle_count += triangle_count;
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

    pub fn fill_cluster_buffers_bind_group_layout(&self) -> BindGroupLayout {
        self.fill_cluster_buffers_bind_group_layout.clone()
    }

    pub fn culling_bind_group_layout(&self) -> BindGroupLayout {
        self.culling_bind_group_layout.clone()
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
    pub second_pass_candidates_buffer: Buffer,
    instance_visibility: Buffer,
    pub visibility_buffer: Option<CachedTexture>,
    pub visibility_buffer_draw_indirect_args_first: Buffer,
    pub visibility_buffer_draw_indirect_args_second: Buffer,
    visibility_buffer_draw_triangle_buffer: Buffer,
    depth_pyramid_all_mips: TextureView,
    depth_pyramid_mips: [TextureView; 12],
    pub depth_pyramid_mip_count: u32,
    previous_depth_pyramid: TextureView,
    pub material_depth_color: Option<CachedTexture>,
    pub material_depth: Option<CachedTexture>,
    pub view_size: UVec2,
}

#[derive(Component)]
pub struct MeshletViewBindGroups {
    pub first_node: Arc<AtomicBool>,
    pub fill_cluster_buffers: BindGroup,
    pub culling_first: BindGroup,
    pub culling_second: BindGroup,
    pub downsample_depth: BindGroup,
    pub visibility_buffer_raster: BindGroup,
    pub copy_material_depth: Option<BindGroup>,
    pub material_draw: Option<BindGroup>,
}
