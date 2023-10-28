use super::{persistent_buffer::PersistentGpuBuffer, Meshlet, MeshletBoundingSphere, MeshletMesh};
use crate::{
    tonemapping_pipeline_key, AlphaMode, EnvironmentMapLight, Material, MaterialPipeline,
    MaterialPipelineKey, MeshFlags, MeshPipelineKey, MeshTransforms, MeshUniform, NotShadowCaster,
    NotShadowReceiver, PreviousGlobalTransform, RenderMaterialInstances, RenderMaterials,
    ScreenSpaceAmbientOcclusionSettings, ShadowFilteringMethod,
};
use bevy_asset::{AssetId, Assets, Handle, UntypedAssetId};
use bevy_core_pipeline::{
    experimental::taa::TemporalAntiAliasSettings,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{
    entity::Entity,
    query::{Has, With},
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    camera::Projection,
    mesh::{InnerMeshVertexBufferLayout, Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    view::{ExtractedView, Msaa, ViewUniforms},
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{tracing::error, EntityHashMap, HashMap};
use std::{
    hash::Hash,
    ops::{DerefMut, Range},
    sync::Arc,
};

pub fn extract_meshlet_meshes(
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
    gpu_scene.reset();

    // TODO: Handle not_shadow_caster
    for (
        instance_index,
        (instance, handle, transform, previous_transform, not_shadow_receiver, _not_shadow_caster),
    ) in query.iter().enumerate()
    {
        gpu_scene.queue_meshlet_mesh_upload(instance, handle, &assets, instance_index as u32);

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

// TODO: Deduplicate view logic shared between many systems
pub fn prepare_material_for_meshlet_meshes<M: Material>(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    mut material_pipeline: ResMut<MaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_materials: Res<RenderMaterials<M>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    images: Res<RenderAssets<Image>>,
    views: Query<(
        Entity,
        &ExtractedView,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&EnvironmentMapLight>,
        Option<&ShadowFilteringMethod>,
        Option<&ScreenSpaceAmbientOcclusionSettings>,
        Has<NormalPrepass>,
        Has<DepthPrepass>,
        Has<MotionVectorPrepass>,
        Has<DeferredPrepass>,
        Option<&TemporalAntiAliasSettings>,
        Option<&Projection>,
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    if material_pipeline.meshlet_layout == None {
        material_pipeline.meshlet_layout = Some(gpu_scene.draw_bind_group_layout().clone());
    }

    let fake_vertex_buffer_layout = &MeshVertexBufferLayout::new(InnerMeshVertexBufferLayout::new(
        vec![
            Mesh::ATTRIBUTE_POSITION.id,
            Mesh::ATTRIBUTE_NORMAL.id,
            Mesh::ATTRIBUTE_UV_0.id,
            Mesh::ATTRIBUTE_TANGENT.id,
        ],
        VertexBufferLayout {
            array_stride: 48,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_POSITION.format,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_NORMAL.format,
                    offset: 12,
                    shader_location: 1,
                },
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_UV_0.format,
                    offset: 24,
                    shader_location: 2,
                },
                VertexAttribute {
                    format: Mesh::ATTRIBUTE_TANGENT.format,
                    offset: 32,
                    shader_location: 3,
                },
            ],
        },
    ));

    for (
        view_entity,
        view,
        tonemapping,
        dither,
        environment_map,
        shadow_filter_method,
        ssao,
        normal_prepass,
        depth_prepass,
        motion_vector_prepass,
        deferred_prepass,
        taa_settings,
        projection,
    ) in &views
    {
        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        if taa_settings.is_some() {
            view_key |= MeshPipelineKey::TAA;
        }
        let environment_map_loaded = environment_map.is_some_and(|map| map.is_loaded(&images));

        if environment_map_loaded {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        if let Some(projection) = projection {
            view_key |= match projection {
                Projection::Perspective(_) => MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
                Projection::Orthographic(_) => MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
            };
        }

        match shadow_filter_method.unwrap_or(&ShadowFilteringMethod::default()) {
            ShadowFilteringMethod::Hardware2x2 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2;
            }
            ShadowFilteringMethod::Castano13 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_CASTANO_13;
            }
            ShadowFilteringMethod::Jimenez14 => {
                view_key |= MeshPipelineKey::SHADOW_FILTER_METHOD_JIMENEZ_14;
            }
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }
        if ssao.is_some() {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        let mut mesh_key = view_key;

        mesh_key |= MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);

        for material_id in render_material_instances.values() {
            let Some(material) = render_materials.get(material_id) else {
                continue;
            };
            if material.properties.alpha_mode != AlphaMode::Opaque {
                // TODO: Log error
                continue;
            }

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &material_pipeline,
                MaterialPipelineKey {
                    mesh_key,
                    for_meshlet_mesh: true,
                    bind_group_data: material.key.clone(),
                },
                fake_vertex_buffer_layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            gpu_scene.materials.insert(
                (material_id.untyped(), view_entity),
                (pipeline_id, material.bind_group.clone()),
            );
        }
    }
}

pub fn determine_meshlet_mesh_material_order(mut gpu_scene: ResMut<MeshletGpuScene>) {
    if gpu_scene.scene_meshlet_count == 0 {
        return;
    }

    let gpu_scene = gpu_scene.deref_mut();

    for (i, (material_id, _)) in gpu_scene.materials.keys().enumerate() {
        gpu_scene.material_order.push(*material_id);
        gpu_scene
            .material_order_lookup
            .insert(*material_id, i as u32);
    }
}

pub fn queue_material_meshlet_meshes<M: Material>(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
) {
    let gpu_scene = gpu_scene.deref_mut();

    for (instance, vertex_count) in &gpu_scene.instances {
        let material_id = render_material_instances
            .get(instance)
            .expect("TODO")
            .untyped();

        *gpu_scene
            .material_vertex_counts
            .entry(material_id)
            .or_default() += *vertex_count;

        gpu_scene.instance_material_ids.get_mut().push(
            *gpu_scene
                .material_order_lookup
                .get(&material_id)
                .expect("TODO: Will this error ever occur?"),
        );
    }
}

pub fn prepare_meshlet_per_frame_resources(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    views: Query<Entity, With<ExtractedView>>,
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
        .instance_material_ids
        .write_buffer(&render_device, &render_queue);
    gpu_scene
        .thread_instance_ids
        .write_buffer(&render_device, &render_queue);
    gpu_scene
        .thread_meshlet_ids
        .write_buffer(&render_device, &render_queue);

    gpu_scene.draw_index_buffer = Some(render_device.create_buffer(&BufferDescriptor {
        label: Some("meshlet_draw_index_buffer"),
        size: 4 * gpu_scene.scene_vertex_count,
        usage: BufferUsages::STORAGE | BufferUsages::INDEX,
        mapped_at_creation: false,
    }));

    for view_entity in &views {
        let mut contents = Vec::new();
        let mut base_index = 0;
        for material in &gpu_scene.material_order {
            contents.extend_from_slice(
                DrawIndexedIndirect {
                    vertex_count: 0,
                    instance_count: 1,
                    base_index,
                    vertex_offset: 0,
                    base_instance: 0,
                }
                .as_bytes(),
            );
            base_index += *gpu_scene.material_vertex_counts.get(material).unwrap_or(&0);
        }

        gpu_scene.draw_command_buffers.insert(
            view_entity,
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("meshlet_draw_command_buffer"),
                contents: &contents,
                usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            }),
        );
    }
}

pub fn prepare_meshlet_per_frame_bind_groups(
    mut gpu_scene: ResMut<MeshletGpuScene>,
    views: Query<Entity, With<ExtractedView>>,
    view_uniforms: Res<ViewUniforms>,
    render_device: Res<RenderDevice>,
) {
    let gpu_scene = gpu_scene.deref_mut();

    let (Some(draw_index_buffer), Some(view_uniforms)) = (
        &gpu_scene.draw_index_buffer,
        view_uniforms.uniforms.binding(),
    ) else {
        return;
    };

    for view_entity in &views {
        let entries = BindGroupEntries::sequential((
            gpu_scene.vertex_data.binding(),
            gpu_scene.vertex_ids.binding(),
            gpu_scene.meshlets.binding(),
            gpu_scene.instance_uniforms.binding().unwrap(),
            gpu_scene.thread_instance_ids.binding().unwrap(),
            gpu_scene.thread_meshlet_ids.binding().unwrap(),
            gpu_scene.instance_material_ids.binding().unwrap(),
            gpu_scene.indices.binding(),
            gpu_scene.meshlet_bounding_spheres.binding(),
            gpu_scene.draw_command_buffers[&view_entity].as_entire_binding(),
            draw_index_buffer.as_entire_binding(),
            view_uniforms.clone(),
        ));

        gpu_scene.culling_bind_groups.insert(
            view_entity,
            render_device.create_bind_group(
                "meshlet_culling_bind_group",
                &gpu_scene.culling_bind_group_layout,
                &entries[2..11],
            ),
        );

        gpu_scene.draw_bind_groups.insert(
            view_entity,
            render_device.create_bind_group(
                "meshlet_draw_bind_group",
                &gpu_scene.draw_bind_group_layout,
                &entries[0..6],
            ),
        );
    }
}

#[derive(Resource)]
pub struct MeshletGpuScene {
    vertex_data: PersistentGpuBuffer<Arc<[u8]>>,
    vertex_ids: PersistentGpuBuffer<Arc<[u32]>>,
    indices: PersistentGpuBuffer<Arc<[u8]>>,
    meshlets: PersistentGpuBuffer<Arc<[Meshlet]>>,
    meshlet_bounding_spheres: PersistentGpuBuffer<Arc<[MeshletBoundingSphere]>>,
    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, (Range<u32>, u64)>,
    materials: HashMap<(UntypedAssetId, Entity), (CachedRenderPipelineId, BindGroup)>,

    scene_meshlet_count: u32,
    scene_vertex_count: u64,
    material_order: Vec<UntypedAssetId>,
    material_order_lookup: HashMap<UntypedAssetId, u32>,
    material_vertex_counts: HashMap<UntypedAssetId, u32>,
    instances: Vec<(Entity, u32)>,
    instance_uniforms: StorageBuffer<Vec<MeshUniform>>,
    instance_material_ids: StorageBuffer<Vec<u32>>,
    thread_instance_ids: StorageBuffer<Vec<u32>>,
    thread_meshlet_ids: StorageBuffer<Vec<u32>>,

    culling_bind_group_layout: BindGroupLayout,
    draw_bind_group_layout: BindGroupLayout,
    draw_command_buffers: EntityHashMap<Entity, Buffer>,
    draw_index_buffer: Option<Buffer>,
    culling_bind_groups: EntityHashMap<Entity, BindGroup>,
    draw_bind_groups: EntityHashMap<Entity, BindGroup>,
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
            materials: HashMap::new(),

            scene_meshlet_count: 0,
            scene_vertex_count: 0,
            material_order: Vec::new(),
            material_order_lookup: HashMap::new(),
            material_vertex_counts: HashMap::new(),
            instances: Vec::new(),
            instance_uniforms: {
                let mut buffer = StorageBuffer::default();
                buffer.set_label(Some("meshlet_instance_uniforms"));
                buffer
            },
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

            culling_bind_group_layout: render_device.create_bind_group_layout(
                &BindGroupLayoutDescriptor {
                    label: Some("meshlet_culling_bind_group_layout"),
                    entries: &bind_group_layout_entries()[2..12],
                },
            ),
            draw_bind_group_layout: render_device.create_bind_group_layout(
                &BindGroupLayoutDescriptor {
                    label: Some("meshlet_draw_bind_group_layout"),
                    entries: &bind_group_layout_entries()[0..6],
                },
            ),
            draw_command_buffers: EntityHashMap::default(),
            draw_index_buffer: None,
            culling_bind_groups: EntityHashMap::default(),
            draw_bind_groups: EntityHashMap::default(),
        }
    }
}

impl MeshletGpuScene {
    fn reset(&mut self) {
        // TODO: Shrink capacity if saturation is low
        self.materials.clear();
        self.scene_meshlet_count = 0;
        self.scene_vertex_count = 0;
        self.material_order.clear();
        self.material_order_lookup.clear();
        self.material_vertex_counts.clear();
        self.instances.clear();
        self.instance_uniforms.get_mut().clear();
        self.instance_material_ids.get_mut().clear();
        self.thread_instance_ids.get_mut().clear();
        self.thread_meshlet_ids.get_mut().clear();
        self.draw_command_buffers.clear();
        self.draw_index_buffer = None;
        self.culling_bind_groups.clear();
        self.draw_bind_groups.clear();
    }

    fn queue_meshlet_mesh_upload(
        &mut self,
        instance: Entity,
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
                    .map(|meshlet| meshlet.triangle_count as u64 * 3)
                    .sum(),
            )
        };

        let (meshlets_slice, vertex_count) = self
            .meshlet_mesh_slices
            .entry(handle.id())
            .or_insert_with_key(queue_meshlet_mesh)
            .clone();

        self.scene_meshlet_count += meshlets_slice.end - meshlets_slice.start;
        self.scene_vertex_count += vertex_count;
        self.instances.push((instance, vertex_count as u32));

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

    pub fn resources_for_view(
        &self,
        view_entity: Entity,
    ) -> (
        u32,
        Vec<Option<&(CachedRenderPipelineId, BindGroup)>>,
        Option<&BindGroup>,
        Option<&BindGroup>,
        Option<&Buffer>,
        Option<&Buffer>,
    ) {
        let mut materials = Vec::new();
        for material_id in &self.material_order {
            materials.push(self.materials.get(&(*material_id, view_entity)));
        }

        (
            self.scene_meshlet_count,
            materials,
            self.culling_bind_groups.get(&view_entity),
            self.draw_bind_groups.get(&view_entity),
            self.draw_index_buffer.as_ref(),
            self.draw_command_buffers.get(&view_entity),
        )
    }
}

fn bind_group_layout_entries() -> [BindGroupLayoutEntry; 12] {
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
        // Instance material IDs
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
        // Indices
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
        // Meshlet bounding spheres
        BindGroupLayoutEntry {
            binding: 8,
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
            binding: 9,
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
            binding: 10,
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
            binding: 11,
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
