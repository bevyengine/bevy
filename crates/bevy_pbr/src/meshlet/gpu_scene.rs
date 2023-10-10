use super::{persistent_buffer::PersistentGpuBuffer, Meshlet, MeshletBoundingSphere, MeshletMesh};
use crate::{
    MeshFlags, MeshTransforms, NotShadowCaster, NotShadowReceiver, PreviousGlobalTransform,
};
use bevy_asset::{AssetId, Assets, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Has,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, BufferBindingType, ShaderStages,
    },
    renderer::{RenderDevice, RenderQueue},
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use std::{ops::Range, sync::Arc};

// TODO: Use ExtractToRenderInstance
// https://github.com/bevyengine/bevy/pull/10002
pub fn extract_meshlet_meshes(
    mut commands: Commands,
    query: Extract<
        Query<(
            Entity,
            &Handle<MeshletMesh>,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            Has<NotShadowReceiver>,
            Has<NotShadowCaster>,
        )>,
    >,
    assets: Extract<Res<Assets<MeshletMesh>>>,
    mut gpu_scene: ResMut<MeshletGpuScene>,
) {
    gpu_scene.total_instanced_meshlet_count = 0;

    for (entity, handle, transform, previous_transform, not_shadow_receiver, not_shadow_caster) in
        &query
    {
        let scene_slice = gpu_scene.queue_meshlet_mesh_upload(handle, &assets);
        // TODO: Unload MeshletMesh asset

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

        commands
            .get_or_spawn(entity)
            .insert((scene_slice, transforms));
    }
}

pub fn perform_pending_meshlet_mesh_writes(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    render_queue: Res<RenderQueue>,
    render_device: Res<RenderDevice>,
) {
    gpu_scene
        .vertex_data
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .meshlet_vertices
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .meshlet_indices
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .meshlets
        .perform_writes(&render_queue, &render_device);
    gpu_scene
        .meshlet_bounding_spheres
        .perform_writes(&render_queue, &render_device);
}

#[derive(Resource)]
pub struct MeshletGpuScene {
    vertex_data: PersistentGpuBuffer<Arc<[u8]>>,
    meshlet_vertices: PersistentGpuBuffer<Arc<[u32]>>,
    meshlet_indices: PersistentGpuBuffer<Arc<[u8]>>,
    meshlets: PersistentGpuBuffer<Arc<[Meshlet]>>,
    meshlet_bounding_spheres: PersistentGpuBuffer<Arc<[MeshletBoundingSphere]>>,

    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, MeshletMeshGpuSceneSlice>,
    total_instanced_meshlet_count: u32,

    bind_group_layout: BindGroupLayout,
}

impl FromWorld for MeshletGpuScene {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        Self {
            vertex_data: PersistentGpuBuffer::new("meshlet_gpu_scene_vertex_data", render_device),
            meshlet_vertices: PersistentGpuBuffer::new(
                "meshlet_gpu_scene_meshlet_vertices",
                render_device,
            ),
            meshlet_indices: PersistentGpuBuffer::new(
                "meshlet_gpu_scene_meshlet_indices",
                render_device,
            ),
            meshlets: PersistentGpuBuffer::new("meshlet_gpu_scene_meshlets", render_device),
            meshlet_bounding_spheres: PersistentGpuBuffer::new(
                "meshlet_gpu_scene_meshlet_bounding_spheres",
                render_device,
            ),

            meshlet_mesh_slices: HashMap::new(),
            total_instanced_meshlet_count: 0,

            bind_group_layout: render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("meshlet_gpu_scene_bind_group_layout"),
                // TODO: min_binding_sizes
                entries: &[
                    // Vertex data
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Meshlet vertices
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Meshlet indices
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Meshlets
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Meshlet bounding spheres
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }),
        }
    }
}

impl MeshletGpuScene {
    fn queue_meshlet_mesh_upload(
        &mut self,
        handle: &Handle<MeshletMesh>,
        assets: &Assets<MeshletMesh>,
    ) -> MeshletMeshGpuSceneSlice {
        let queue_meshlet_mesh = |asset_id: &AssetId<MeshletMesh>| {
            let meshlet_mesh = assets.get(*asset_id).expect("TODO");

            let vertex_data_slice = self
                .vertex_data
                .queue_write(Arc::clone(&meshlet_mesh.vertex_data), ());
            let meshlet_vertices_slice = self.meshlet_vertices.queue_write(
                Arc::clone(&meshlet_mesh.meshlet_vertices),
                vertex_data_slice.start,
            );
            let meshlet_indices_slice = self
                .meshlet_indices
                .queue_write(Arc::clone(&meshlet_mesh.meshlet_indices), ());
            let meshlet_slice = self.meshlets.queue_write(
                Arc::clone(&meshlet_mesh.meshlets),
                (meshlet_vertices_slice.start, meshlet_indices_slice.start),
            );
            self.meshlet_bounding_spheres
                .queue_write(Arc::clone(&meshlet_mesh.meshlet_bounding_spheres), ());

            MeshletMeshGpuSceneSlice(
                (meshlet_slice.start as u32 / 16)..(meshlet_slice.end as u32 / 16),
            )
        };

        let scene_slice = self
            .meshlet_mesh_slices
            .entry(handle.id())
            .or_insert_with_key(queue_meshlet_mesh)
            .clone();

        self.total_instanced_meshlet_count += scene_slice.0.end - scene_slice.0.start;

        scene_slice
    }

    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn create_bind_group(&self, render_device: &RenderDevice) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("meshlet_gpu_scene_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.vertex_data.binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: self.meshlet_vertices.binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.meshlet_indices.binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.meshlets.binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: self.meshlet_bounding_spheres.binding(),
                },
            ],
        })
    }

    pub fn total_instanced_meshlet_count(&self) -> u32 {
        self.total_instanced_meshlet_count
    }
}

#[derive(Component, Clone)]
pub struct MeshletMeshGpuSceneSlice(pub Range<u32>);
