use super::{persistent_buffer::PersistentGpuBuffer, Meshlet, MeshletBoundingSphere, MeshletMesh};
use crate::{
    MeshFlags, MeshTransforms, MeshUniform, NotShadowCaster, NotShadowReceiver,
    PreviousGlobalTransform,
};
use bevy_asset::{AssetId, Assets, Handle};
use bevy_ecs::{
    query::Has,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    view::ViewUniforms,
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use std::{ops::Range, sync::Arc};

pub fn extract_meshlet_meshes(
    query: Extract<
        Query<(
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
    gpu_scene.reset();

    // TODO: Handle not_shadow_caster
    for (
        instance_index,
        (handle, transform, previous_transform, not_shadow_receiver, _not_shadow_caster),
    ) in query.iter().enumerate()
    {
        gpu_scene.queue_meshlet_mesh_upload(handle, &assets, instance_index as u32);

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
        gpu_scene
            .instance_uniforms
            .get_mut()
            .push(MeshUniform::from(&transforms));
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

pub fn prepare_meshlet_per_frame_resources(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    render_queue: Res<RenderQueue>,
    render_device: Res<RenderDevice>,
) {
    if gpu_scene.scene_meshlet_count == 0 {
        return;
    }

    gpu_scene
        .instance_uniforms
        .write_buffer(&render_device, &render_queue);
    gpu_scene
        .thread_instance_ids
        .write_buffer(&render_device, &render_queue);
    gpu_scene
        .thread_meshlet_ids
        .write_buffer(&render_device, &render_queue);

    gpu_scene.draw_command_buffer = Some(
        render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("meshlet_draw_command_buffer"),
            contents: DrawIndexedIndirect {
                vertex_count: 0,
                instance_count: 1,
                base_index: 0,
                vertex_offset: 0,
                base_instance: 0,
            }
            .as_bytes(),
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
        }),
    );

    gpu_scene.draw_index_buffer = Some(render_device.create_buffer(&BufferDescriptor {
        label: Some("meshlet_draw_index_buffer"),
        size: 12 * gpu_scene.scene_triangle_count,
        usage: BufferUsages::STORAGE | BufferUsages::INDEX,
        mapped_at_creation: false,
    }));
}

pub fn prepare_meshlet_per_frame_bind_groups(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    view_uniforms: Res<ViewUniforms>,
    render_device: Res<RenderDevice>,
) {
    if gpu_scene.scene_meshlet_count == 0 {
        return;
    }
    let Some(view_uniforms) = view_uniforms.uniforms.binding() else {
        return;
    };

    let entries = &[
        BindGroupEntry {
            binding: 0,
            resource: gpu_scene.vertex_data.binding(),
        },
        BindGroupEntry {
            binding: 1,
            resource: gpu_scene.vertex_ids.binding(),
        },
        BindGroupEntry {
            binding: 2,
            resource: gpu_scene.meshlets.binding(),
        },
        BindGroupEntry {
            binding: 3,
            resource: gpu_scene.instance_uniforms.binding().unwrap(),
        },
        BindGroupEntry {
            binding: 4,
            resource: gpu_scene.thread_instance_ids.binding().unwrap(),
        },
        BindGroupEntry {
            binding: 5,
            resource: gpu_scene.thread_meshlet_ids.binding().unwrap(),
        },
        BindGroupEntry {
            binding: 6,
            resource: gpu_scene.indices.binding(),
        },
        BindGroupEntry {
            binding: 7,
            resource: gpu_scene.meshlet_bounding_spheres.binding(),
        },
        BindGroupEntry {
            binding: 8,
            resource: gpu_scene
                .draw_command_buffer
                .as_ref()
                .unwrap()
                .as_entire_binding(),
        },
        BindGroupEntry {
            binding: 9,
            resource: gpu_scene
                .draw_index_buffer
                .as_ref()
                .unwrap()
                .as_entire_binding(),
        },
        BindGroupEntry {
            binding: 10,
            resource: view_uniforms,
        },
    ];

    let culling_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("meshlet_culling_bind_group"),
        layout: &gpu_scene.culling_bind_group_layout,
        entries: &entries[2..11],
    });
    let draw_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("meshlet_draw_bind_group"),
        layout: &gpu_scene.draw_bind_group_layout,
        entries: &entries[0..6],
    });

    gpu_scene.culling_bind_group = Some(culling_bind_group);
    gpu_scene.draw_bind_group = Some(draw_bind_group);
}

#[derive(Resource)]
pub struct MeshletGpuScene {
    vertex_data: PersistentGpuBuffer<Arc<[u8]>>,
    vertex_ids: PersistentGpuBuffer<Arc<[u32]>>,
    indices: PersistentGpuBuffer<Arc<[u8]>>,
    meshlets: PersistentGpuBuffer<Arc<[Meshlet]>>,
    meshlet_bounding_spheres: PersistentGpuBuffer<Arc<[MeshletBoundingSphere]>>,
    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, (Range<u32>, u32)>,

    scene_meshlet_count: u32,
    scene_triangle_count: u64,
    instance_uniforms: StorageBuffer<Vec<MeshUniform>>,
    thread_instance_ids: StorageBuffer<Vec<u32>>,
    thread_meshlet_ids: StorageBuffer<Vec<u32>>,

    culling_bind_group_layout: BindGroupLayout,
    draw_bind_group_layout: BindGroupLayout,

    draw_command_buffer: Option<Buffer>,
    draw_index_buffer: Option<Buffer>,
    culling_bind_group: Option<BindGroup>,
    draw_bind_group: Option<BindGroup>,
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
            instance_uniforms: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_uniforms"));
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

            culling_bind_group_layout: render_device.create_bind_group_layout(
                &BindGroupLayoutDescriptor {
                    label: Some("meshlet_culling_bind_group_layout"),
                    entries: &bind_group_layout_entries()[2..11],
                },
            ),
            draw_bind_group_layout: render_device.create_bind_group_layout(
                &BindGroupLayoutDescriptor {
                    label: Some("meshlet_draw_bind_group_layout"),
                    entries: &bind_group_layout_entries()[0..6],
                },
            ),

            draw_command_buffer: None,
            draw_index_buffer: None,
            culling_bind_group: None,
            draw_bind_group: None,
        }
    }
}

impl MeshletGpuScene {
    fn reset(&mut self) {
        self.scene_meshlet_count = 0;
        self.scene_triangle_count = 0;
        // TODO: Shrink capacity if saturation is low
        self.instance_uniforms.get_mut().clear();
        self.thread_instance_ids.get_mut().clear();
        self.thread_meshlet_ids.get_mut().clear();
        self.draw_command_buffer = None;
        self.draw_index_buffer = None;
        self.culling_bind_group = None;
        self.draw_bind_group = None;
    }

    fn queue_meshlet_mesh_upload(
        &mut self,
        handle: &Handle<MeshletMesh>,
        assets: &Assets<MeshletMesh>,
        instance_index: u32,
    ) {
        let queue_meshlet_mesh = |asset_id: &AssetId<MeshletMesh>| {
            let meshlet_mesh = assets.get(*asset_id).expect("TODO");

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
            self.meshlet_bounding_spheres
                .queue_write(Arc::clone(&meshlet_mesh.meshlet_bounding_spheres), ());

            (
                (meshlets_slice.start as u32 / 16)..(meshlets_slice.end as u32 / 16),
                // TODO: Pre-compute this value during conversion and store in MeshletMesh
                meshlet_mesh
                    .meshlets
                    .iter()
                    .map(|meshlet| meshlet.triangle_count)
                    .sum(),
            )
        };

        let (meshlets_slice, triangle_count) = self
            .meshlet_mesh_slices
            .entry(handle.id())
            .or_insert_with_key(queue_meshlet_mesh)
            .clone();

        self.scene_meshlet_count += meshlets_slice.end - meshlets_slice.start;
        self.scene_triangle_count += triangle_count as u64;

        for meshlet_index in meshlets_slice {
            self.thread_instance_ids.get_mut().push(instance_index);
            self.thread_meshlet_ids.get_mut().push(meshlet_index);
        }
    }

    pub fn culling_bind_group_layout(&self) -> &BindGroupLayout {
        &self.culling_bind_group_layout
    }

    pub fn draw_bind_group_layout(&self) -> &BindGroupLayout {
        &self.draw_bind_group_layout
    }

    pub fn resources(
        &self,
    ) -> (
        u32,
        Option<&BindGroup>,
        Option<&BindGroup>,
        Option<&Buffer>,
        Option<&Buffer>,
    ) {
        (
            self.scene_meshlet_count,
            self.culling_bind_group.as_ref(),
            self.draw_bind_group.as_ref(),
            self.draw_index_buffer.as_ref(),
            self.draw_command_buffer.as_ref(),
        )
    }
}

fn bind_group_layout_entries() -> [BindGroupLayoutEntry; 11] {
    // TODO: min_binding_size
    [
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
        // Vertex IDs
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
        // Meshlets
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Instance uniforms
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Thread instance IDs
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Thread meshlet IDs
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Indices
        BindGroupLayoutEntry {
            binding: 6,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Meshlet bounding spheres
        BindGroupLayoutEntry {
            binding: 7,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Draw command buffer
        BindGroupLayoutEntry {
            binding: 8,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Draw index buffer
        BindGroupLayoutEntry {
            binding: 9,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // View
        BindGroupLayoutEntry {
            binding: 10,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            count: None,
        },
    ]
}
