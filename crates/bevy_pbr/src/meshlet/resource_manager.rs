use super::{instance_manager::InstanceManager, meshlet_mesh_manager::MeshletMeshManager};
use crate::ShadowView;
use alloc::sync::Arc;
use bevy_core_pipeline::{
    core_3d::Camera3d,
    experimental::mip_generation::{self, ViewDepthPyramid},
    prepass::{PreviousViewData, PreviousViewUniforms},
};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashMap},
    query::AnyOf,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::{UVec2, Vec4Swizzles};
use bevy_render::{
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, RenderLayers, ViewUniform, ViewUniforms},
};
use binding_types::*;
use core::{iter, sync::atomic::AtomicBool};
use encase::internal::WriteInto;

/// Manages per-view and per-cluster GPU resources for [`super::MeshletPlugin`].
#[derive(Resource)]
pub struct ResourceManager {
    /// Intermediate buffer of cluster IDs for use with rasterizing the visibility buffer
    visibility_buffer_raster_clusters: Buffer,
    /// Intermediate buffer of count of clusters to software rasterize
    software_raster_cluster_count: Buffer,
    /// Rightmost slot index of [`Self::visibility_buffer_raster_clusters`]
    raster_cluster_rightmost_slot: u32,

    /// Per-cluster instance ID
    cluster_instance_ids: Option<Buffer>,
    /// Per-cluster meshlet ID
    cluster_meshlet_ids: Option<Buffer>,
    /// Per-cluster bitmask of whether or not it's a candidate for the second raster pass
    second_pass_candidates_buffer: Option<Buffer>,
    /// Sampler for a depth pyramid
    depth_pyramid_sampler: Sampler,
    /// Dummy texture view for binding depth pyramids with less than the maximum amount of mips
    depth_pyramid_dummy_texture: TextureView,

    // TODO
    previous_depth_pyramids: EntityHashMap<TextureView>,

    // Bind group layouts
    pub fill_cluster_buffers_bind_group_layout: BindGroupLayout,
    pub clear_visibility_buffer_bind_group_layout: BindGroupLayout,
    pub clear_visibility_buffer_shadow_view_bind_group_layout: BindGroupLayout,
    pub culling_bind_group_layout: BindGroupLayout,
    pub visibility_buffer_raster_bind_group_layout: BindGroupLayout,
    pub visibility_buffer_raster_shadow_view_bind_group_layout: BindGroupLayout,
    pub downsample_depth_bind_group_layout: BindGroupLayout,
    pub downsample_depth_shadow_view_bind_group_layout: BindGroupLayout,
    pub resolve_depth_bind_group_layout: BindGroupLayout,
    pub resolve_depth_shadow_view_bind_group_layout: BindGroupLayout,
    pub resolve_material_depth_bind_group_layout: BindGroupLayout,
    pub material_shade_bind_group_layout: BindGroupLayout,
    pub remap_1d_to_2d_dispatch_bind_group_layout: Option<BindGroupLayout>,
}

impl ResourceManager {
    pub fn new(cluster_buffer_slots: u32, render_device: &RenderDevice) -> Self {
        let needs_dispatch_remap =
            cluster_buffer_slots > render_device.limits().max_compute_workgroups_per_dimension;

        Self {
            visibility_buffer_raster_clusters: render_device.create_buffer(&BufferDescriptor {
                label: Some("meshlet_visibility_buffer_raster_clusters"),
                size: cluster_buffer_slots as u64 * size_of::<u32>() as u64,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            software_raster_cluster_count: render_device.create_buffer(&BufferDescriptor {
                label: Some("meshlet_software_raster_cluster_count"),
                size: size_of::<u32>() as u64,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            raster_cluster_rightmost_slot: cluster_buffer_slots - 1,

            cluster_instance_ids: None,
            cluster_meshlet_ids: None,
            second_pass_candidates_buffer: None,
            depth_pyramid_sampler: render_device.create_sampler(&SamplerDescriptor {
                label: Some("meshlet_depth_pyramid_sampler"),
                ..SamplerDescriptor::default()
            }),
            depth_pyramid_dummy_texture: mip_generation::create_depth_pyramid_dummy_texture(
                render_device,
                "meshlet_depth_pyramid_dummy_texture",
                "meshlet_depth_pyramid_dummy_texture_view",
            ),

            previous_depth_pyramids: EntityHashMap::default(),

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
                        storage_buffer_sized(false, None),
                    ),
                ),
            ),
            clear_visibility_buffer_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_clear_visibility_buffer_bind_group_layout",
                &BindGroupLayoutEntries::single(
                    ShaderStages::COMPUTE,
                    texture_storage_2d(TextureFormat::R64Uint, StorageTextureAccess::WriteOnly),
                ),
            ),
            clear_visibility_buffer_shadow_view_bind_group_layout: render_device
                .create_bind_group_layout(
                    "meshlet_clear_visibility_buffer_shadow_view_bind_group_layout",
                    &BindGroupLayoutEntries::single(
                        ShaderStages::COMPUTE,
                        texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::WriteOnly),
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
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_sized(false, None),
                        storage_buffer_sized(false, None),
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
                        texture_storage_2d(TextureFormat::R64Uint, StorageTextureAccess::ReadOnly),
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
            downsample_depth_shadow_view_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_downsample_depth_shadow_view_bind_group_layout",
                &BindGroupLayoutEntries::sequential(ShaderStages::COMPUTE, {
                    let write_only_r32float = || {
                        texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly)
                    };
                    (
                        texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::ReadOnly),
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
                    ShaderStages::all(),
                    (
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        texture_storage_2d(TextureFormat::R64Uint, StorageTextureAccess::Atomic),
                        uniform_buffer::<ViewUniform>(true),
                    ),
                ),
            ),
            visibility_buffer_raster_shadow_view_bind_group_layout: render_device
                .create_bind_group_layout(
                    "meshlet_visibility_buffer_raster_shadow_view_bind_group_layout",
                    &BindGroupLayoutEntries::sequential(
                        ShaderStages::all(),
                        (
                            storage_buffer_read_only_sized(false, None),
                            storage_buffer_read_only_sized(false, None),
                            storage_buffer_read_only_sized(false, None),
                            storage_buffer_read_only_sized(false, None),
                            storage_buffer_read_only_sized(false, None),
                            storage_buffer_read_only_sized(false, None),
                            storage_buffer_read_only_sized(false, None),
                            storage_buffer_read_only_sized(false, None),
                            texture_storage_2d(
                                TextureFormat::R32Uint,
                                StorageTextureAccess::Atomic,
                            ),
                            uniform_buffer::<ViewUniform>(true),
                        ),
                    ),
                ),
            resolve_depth_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_resolve_depth_bind_group_layout",
                &BindGroupLayoutEntries::single(
                    ShaderStages::FRAGMENT,
                    texture_storage_2d(TextureFormat::R64Uint, StorageTextureAccess::ReadOnly),
                ),
            ),
            resolve_depth_shadow_view_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_resolve_depth_shadow_view_bind_group_layout",
                &BindGroupLayoutEntries::single(
                    ShaderStages::FRAGMENT,
                    texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::ReadOnly),
                ),
            ),
            resolve_material_depth_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_resolve_material_depth_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_storage_2d(TextureFormat::R64Uint, StorageTextureAccess::ReadOnly),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                    ),
                ),
            ),
            material_shade_bind_group_layout: render_device.create_bind_group_layout(
                "meshlet_mesh_material_shade_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_storage_2d(TextureFormat::R64Uint, StorageTextureAccess::ReadOnly),
                        storage_buffer_read_only_sized(false, None),
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
            remap_1d_to_2d_dispatch_bind_group_layout: needs_dispatch_remap.then(|| {
                render_device.create_bind_group_layout(
                    "meshlet_remap_1d_to_2d_dispatch_bind_group_layout",
                    &BindGroupLayoutEntries::sequential(
                        ShaderStages::COMPUTE,
                        (
                            storage_buffer_sized(false, None),
                            storage_buffer_sized(false, None),
                        ),
                    ),
                )
            }),
        }
    }
}

// ------------ TODO: Everything under here needs to be rewritten and cached ------------

#[derive(Component)]
pub struct MeshletViewResources {
    pub scene_instance_count: u32,
    pub scene_cluster_count: u32,
    pub second_pass_candidates_buffer: Buffer,
    instance_visibility: Buffer,
    pub dummy_render_target: CachedTexture,
    pub visibility_buffer: CachedTexture,
    pub visibility_buffer_software_raster_indirect_args_first: Buffer,
    pub visibility_buffer_software_raster_indirect_args_second: Buffer,
    pub visibility_buffer_hardware_raster_indirect_args_first: Buffer,
    pub visibility_buffer_hardware_raster_indirect_args_second: Buffer,
    pub depth_pyramid: ViewDepthPyramid,
    previous_depth_pyramid: TextureView,
    pub material_depth: Option<CachedTexture>,
    pub view_size: UVec2,
    pub raster_cluster_rightmost_slot: u32,
    not_shadow_view: bool,
}

#[derive(Component)]
pub struct MeshletViewBindGroups {
    pub first_node: Arc<AtomicBool>,
    pub fill_cluster_buffers: BindGroup,
    pub clear_visibility_buffer: BindGroup,
    pub culling_first: BindGroup,
    pub culling_second: BindGroup,
    pub downsample_depth: BindGroup,
    pub visibility_buffer_raster: BindGroup,
    pub resolve_depth: BindGroup,
    pub resolve_material_depth: Option<BindGroup>,
    pub material_shade: Option<BindGroup>,
    pub remap_1d_to_2d_dispatch: Option<(BindGroup, BindGroup)>,
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

// TODO: Cache things per-view and skip running this system / optimize this system
pub fn prepare_meshlet_per_frame_resources(
    mut resource_manager: ResMut<ResourceManager>,
    mut instance_manager: ResMut<InstanceManager>,
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
    if instance_manager.scene_cluster_count == 0 {
        return;
    }

    let instance_manager = instance_manager.as_mut();

    // TODO: Move this and the submit to a separate system and remove pub from the fields
    instance_manager
        .instance_uniforms
        .write_buffer(&render_device, &render_queue);
    upload_storage_buffer(
        &mut instance_manager.instance_material_ids,
        &render_device,
        &render_queue,
    );
    upload_storage_buffer(
        &mut instance_manager.instance_meshlet_counts,
        &render_device,
        &render_queue,
    );
    upload_storage_buffer(
        &mut instance_manager.instance_meshlet_slice_starts,
        &render_device,
        &render_queue,
    );

    let needed_buffer_size = 4 * instance_manager.scene_cluster_count as u64;
    match &mut resource_manager.cluster_instance_ids {
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
    match &mut resource_manager.cluster_meshlet_ids {
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

    let needed_buffer_size =
        instance_manager.scene_cluster_count.div_ceil(u32::BITS) as u64 * size_of::<u32>() as u64;
    for (view_entity, view, render_layers, (_, shadow_view)) in &views {
        let not_shadow_view = shadow_view.is_none();

        let instance_visibility = instance_manager
            .view_instance_visibility
            .entry(view_entity)
            .or_insert_with(|| {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_view_instance_visibility"));
                buffer
            });
        for (instance_index, (_, layers, not_shadow_caster)) in
            instance_manager.instances.iter().enumerate()
        {
            // If either the layers don't match the view's layers or this is a shadow view
            // and the instance is not a shadow caster, hide the instance for this view
            if !render_layers
                .unwrap_or(&RenderLayers::default())
                .intersects(layers)
                || (shadow_view.is_some() && *not_shadow_caster)
            {
                let vec = instance_visibility.get_mut();
                let index = instance_index / 32;
                let bit = instance_index - index * 32;
                if vec.len() <= index {
                    vec.extend(iter::repeat_n(0, index - vec.len() + 1));
                }
                vec[index] |= 1 << bit;
            }
        }
        upload_storage_buffer(instance_visibility, &render_device, &render_queue);
        let instance_visibility = instance_visibility.buffer().unwrap().clone();

        let second_pass_candidates_buffer =
            match &mut resource_manager.second_pass_candidates_buffer {
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

        // TODO: Remove this once wgpu allows render passes with no attachments
        let dummy_render_target = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("meshlet_dummy_render_target"),
                size: Extent3d {
                    width: view.viewport.z,
                    height: view.viewport.w,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Uint,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        );

        let visibility_buffer = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("meshlet_visibility_buffer"),
                size: Extent3d {
                    width: view.viewport.z,
                    height: view.viewport.w,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if not_shadow_view {
                    TextureFormat::R64Uint
                } else {
                    TextureFormat::R32Uint
                },
                usage: TextureUsages::STORAGE_ATOMIC | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            },
        );

        let visibility_buffer_software_raster_indirect_args_first = render_device
            .create_buffer_with_data(&BufferInitDescriptor {
                label: Some("meshlet_visibility_buffer_software_raster_indirect_args_first"),
                contents: DispatchIndirectArgs { x: 0, y: 1, z: 1 }.as_bytes(),
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            });
        let visibility_buffer_software_raster_indirect_args_second = render_device
            .create_buffer_with_data(&BufferInitDescriptor {
                label: Some("visibility_buffer_software_raster_indirect_args_second"),
                contents: DispatchIndirectArgs { x: 0, y: 1, z: 1 }.as_bytes(),
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            });

        let visibility_buffer_hardware_raster_indirect_args_first = render_device
            .create_buffer_with_data(&BufferInitDescriptor {
                label: Some("meshlet_visibility_buffer_hardware_raster_indirect_args_first"),
                contents: DrawIndirectArgs {
                    vertex_count: 128 * 3,
                    instance_count: 0,
                    first_vertex: 0,
                    first_instance: 0,
                }
                .as_bytes(),
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            });
        let visibility_buffer_hardware_raster_indirect_args_second = render_device
            .create_buffer_with_data(&BufferInitDescriptor {
                label: Some("visibility_buffer_hardware_raster_indirect_args_second"),
                contents: DrawIndirectArgs {
                    vertex_count: 128 * 3,
                    instance_count: 0,
                    first_vertex: 0,
                    first_instance: 0,
                }
                .as_bytes(),
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            });

        let depth_pyramid = ViewDepthPyramid::new(
            &render_device,
            &mut texture_cache,
            &resource_manager.depth_pyramid_dummy_texture,
            view.viewport.zw(),
            "meshlet_depth_pyramid",
            "meshlet_depth_pyramid_texture_view",
        );

        let previous_depth_pyramid =
            match resource_manager.previous_depth_pyramids.get(&view_entity) {
                Some(texture_view) => texture_view.clone(),
                None => depth_pyramid.all_mips.clone(),
            };
        resource_manager
            .previous_depth_pyramids
            .insert(view_entity, depth_pyramid.all_mips.clone());

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

        commands.entity(view_entity).insert(MeshletViewResources {
            scene_instance_count: instance_manager.scene_instance_count,
            scene_cluster_count: instance_manager.scene_cluster_count,
            second_pass_candidates_buffer,
            instance_visibility,
            dummy_render_target,
            visibility_buffer,
            visibility_buffer_software_raster_indirect_args_first,
            visibility_buffer_software_raster_indirect_args_second,
            visibility_buffer_hardware_raster_indirect_args_first,
            visibility_buffer_hardware_raster_indirect_args_second,
            depth_pyramid,
            previous_depth_pyramid,
            material_depth: not_shadow_view
                .then(|| texture_cache.get(&render_device, material_depth)),
            view_size: view.viewport.zw(),
            raster_cluster_rightmost_slot: resource_manager.raster_cluster_rightmost_slot,
            not_shadow_view,
        });
    }
}

pub fn prepare_meshlet_view_bind_groups(
    meshlet_mesh_manager: Res<MeshletMeshManager>,
    resource_manager: Res<ResourceManager>,
    instance_manager: Res<InstanceManager>,
    views: Query<(Entity, &MeshletViewResources)>,
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
        resource_manager.cluster_instance_ids.as_ref(),
        resource_manager.cluster_meshlet_ids.as_ref(),
        view_uniforms.uniforms.binding(),
        previous_view_uniforms.uniforms.binding(),
    )
    else {
        return;
    };

    let first_node = Arc::new(AtomicBool::new(true));

    let fill_cluster_buffers_global_cluster_count =
        render_device.create_buffer(&BufferDescriptor {
            label: Some("meshlet_fill_cluster_buffers_global_cluster_count"),
            size: 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

    // TODO: Some of these bind groups can be reused across multiple views
    for (view_entity, view_resources) in &views {
        let entries = BindGroupEntries::sequential((
            instance_manager.instance_meshlet_counts.binding().unwrap(),
            instance_manager
                .instance_meshlet_slice_starts
                .binding()
                .unwrap(),
            cluster_instance_ids.as_entire_binding(),
            cluster_meshlet_ids.as_entire_binding(),
            fill_cluster_buffers_global_cluster_count.as_entire_binding(),
        ));
        let fill_cluster_buffers = render_device.create_bind_group(
            "meshlet_fill_cluster_buffers",
            &resource_manager.fill_cluster_buffers_bind_group_layout,
            &entries,
        );

        let clear_visibility_buffer = render_device.create_bind_group(
            "meshlet_clear_visibility_buffer_bind_group",
            if view_resources.not_shadow_view {
                &resource_manager.clear_visibility_buffer_bind_group_layout
            } else {
                &resource_manager.clear_visibility_buffer_shadow_view_bind_group_layout
            },
            &BindGroupEntries::single(&view_resources.visibility_buffer.default_view),
        );

        let entries = BindGroupEntries::sequential((
            cluster_meshlet_ids.as_entire_binding(),
            meshlet_mesh_manager.meshlet_bounding_spheres.binding(),
            meshlet_mesh_manager.meshlet_simplification_errors.binding(),
            cluster_instance_ids.as_entire_binding(),
            instance_manager.instance_uniforms.binding().unwrap(),
            view_resources.instance_visibility.as_entire_binding(),
            view_resources
                .second_pass_candidates_buffer
                .as_entire_binding(),
            view_resources
                .visibility_buffer_software_raster_indirect_args_first
                .as_entire_binding(),
            view_resources
                .visibility_buffer_hardware_raster_indirect_args_first
                .as_entire_binding(),
            resource_manager
                .visibility_buffer_raster_clusters
                .as_entire_binding(),
            &view_resources.previous_depth_pyramid,
            view_uniforms.clone(),
            previous_view_uniforms.clone(),
        ));
        let culling_first = render_device.create_bind_group(
            "meshlet_culling_first_bind_group",
            &resource_manager.culling_bind_group_layout,
            &entries,
        );

        let entries = BindGroupEntries::sequential((
            cluster_meshlet_ids.as_entire_binding(),
            meshlet_mesh_manager.meshlet_bounding_spheres.binding(),
            meshlet_mesh_manager.meshlet_simplification_errors.binding(),
            cluster_instance_ids.as_entire_binding(),
            instance_manager.instance_uniforms.binding().unwrap(),
            view_resources.instance_visibility.as_entire_binding(),
            view_resources
                .second_pass_candidates_buffer
                .as_entire_binding(),
            view_resources
                .visibility_buffer_software_raster_indirect_args_second
                .as_entire_binding(),
            view_resources
                .visibility_buffer_hardware_raster_indirect_args_second
                .as_entire_binding(),
            resource_manager
                .visibility_buffer_raster_clusters
                .as_entire_binding(),
            &view_resources.depth_pyramid.all_mips,
            view_uniforms.clone(),
            previous_view_uniforms.clone(),
        ));
        let culling_second = render_device.create_bind_group(
            "meshlet_culling_second_bind_group",
            &resource_manager.culling_bind_group_layout,
            &entries,
        );

        let downsample_depth = view_resources.depth_pyramid.create_bind_group(
            &render_device,
            "meshlet_downsample_depth_bind_group",
            if view_resources.not_shadow_view {
                &resource_manager.downsample_depth_bind_group_layout
            } else {
                &resource_manager.downsample_depth_shadow_view_bind_group_layout
            },
            &view_resources.visibility_buffer.default_view,
            &resource_manager.depth_pyramid_sampler,
        );

        let entries = BindGroupEntries::sequential((
            cluster_meshlet_ids.as_entire_binding(),
            meshlet_mesh_manager.meshlets.binding(),
            meshlet_mesh_manager.indices.binding(),
            meshlet_mesh_manager.vertex_positions.binding(),
            cluster_instance_ids.as_entire_binding(),
            instance_manager.instance_uniforms.binding().unwrap(),
            resource_manager
                .visibility_buffer_raster_clusters
                .as_entire_binding(),
            resource_manager
                .software_raster_cluster_count
                .as_entire_binding(),
            &view_resources.visibility_buffer.default_view,
            view_uniforms.clone(),
        ));
        let visibility_buffer_raster = render_device.create_bind_group(
            "meshlet_visibility_raster_buffer_bind_group",
            if view_resources.not_shadow_view {
                &resource_manager.visibility_buffer_raster_bind_group_layout
            } else {
                &resource_manager.visibility_buffer_raster_shadow_view_bind_group_layout
            },
            &entries,
        );

        let resolve_depth = render_device.create_bind_group(
            "meshlet_resolve_depth_bind_group",
            if view_resources.not_shadow_view {
                &resource_manager.resolve_depth_bind_group_layout
            } else {
                &resource_manager.resolve_depth_shadow_view_bind_group_layout
            },
            &BindGroupEntries::single(&view_resources.visibility_buffer.default_view),
        );

        let resolve_material_depth = view_resources.material_depth.as_ref().map(|_| {
            let entries = BindGroupEntries::sequential((
                &view_resources.visibility_buffer.default_view,
                cluster_instance_ids.as_entire_binding(),
                instance_manager.instance_material_ids.binding().unwrap(),
            ));
            render_device.create_bind_group(
                "meshlet_resolve_material_depth_bind_group",
                &resource_manager.resolve_material_depth_bind_group_layout,
                &entries,
            )
        });

        let material_shade = view_resources.material_depth.as_ref().map(|_| {
            let entries = BindGroupEntries::sequential((
                &view_resources.visibility_buffer.default_view,
                cluster_meshlet_ids.as_entire_binding(),
                meshlet_mesh_manager.meshlets.binding(),
                meshlet_mesh_manager.indices.binding(),
                meshlet_mesh_manager.vertex_positions.binding(),
                meshlet_mesh_manager.vertex_normals.binding(),
                meshlet_mesh_manager.vertex_uvs.binding(),
                cluster_instance_ids.as_entire_binding(),
                instance_manager.instance_uniforms.binding().unwrap(),
            ));
            render_device.create_bind_group(
                "meshlet_mesh_material_shade_bind_group",
                &resource_manager.material_shade_bind_group_layout,
                &entries,
            )
        });

        let remap_1d_to_2d_dispatch = resource_manager
            .remap_1d_to_2d_dispatch_bind_group_layout
            .as_ref()
            .map(|layout| {
                (
                    render_device.create_bind_group(
                        "meshlet_remap_1d_to_2d_dispatch_first_bind_group",
                        layout,
                        &BindGroupEntries::sequential((
                            view_resources
                                .visibility_buffer_software_raster_indirect_args_first
                                .as_entire_binding(),
                            resource_manager
                                .software_raster_cluster_count
                                .as_entire_binding(),
                        )),
                    ),
                    render_device.create_bind_group(
                        "meshlet_remap_1d_to_2d_dispatch_second_bind_group",
                        layout,
                        &BindGroupEntries::sequential((
                            view_resources
                                .visibility_buffer_software_raster_indirect_args_second
                                .as_entire_binding(),
                            resource_manager
                                .software_raster_cluster_count
                                .as_entire_binding(),
                        )),
                    ),
                )
            });

        commands.entity(view_entity).insert(MeshletViewBindGroups {
            first_node: Arc::clone(&first_node),
            fill_cluster_buffers,
            clear_visibility_buffer,
            culling_first,
            culling_second,
            downsample_depth,
            visibility_buffer_raster,
            resolve_depth,
            resolve_material_depth,
            material_shade,
            remap_1d_to_2d_dispatch,
        });
    }
}
