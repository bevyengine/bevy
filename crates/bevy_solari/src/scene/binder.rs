use super::{blas::BlasManager, extract::StandardMaterialAssets, RaytracingMesh3d};
use bevy_asset::{AssetId, Handle};
use bevy_color::LinearRgba;
use bevy_ecs::{
    resource::Resource,
    system::{Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::{Mat4, UVec4};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_platform_support::{collections::HashMap, hash::FixedHasher};
use bevy_render::{
    mesh::allocator::MeshAllocator,
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{FallbackImage, GpuImage},
};
use bevy_transform::components::GlobalTransform;
use std::{hash::Hash, num::NonZeroU32, ops::Deref};

const MAX_MESH_COUNT: Option<NonZeroU32> = NonZeroU32::new(5_000);
const MAX_TEXTURE_COUNT: Option<NonZeroU32> = NonZeroU32::new(5_000);

#[derive(Resource)]
pub struct RaytracingSceneBindings {
    pub bind_group: Option<BindGroup>,
    pub bind_group_layout: BindGroupLayout,
}

pub fn prepare_raytracing_scene_bindings(
    instances: Query<(
        &RaytracingMesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &GlobalTransform,
    )>,
    mesh_allocator: Res<MeshAllocator>,
    blas_manager: Res<BlasManager>,
    material_assets: Res<StandardMaterialAssets>,
    texture_assets: Res<RenderAssets<GpuImage>>,
    fallback_texture: Res<FallbackImage>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut raytracing_scene_bindings: ResMut<RaytracingSceneBindings>,
) {
    raytracing_scene_bindings.bind_group = None;

    if instances.iter().len() == 0 {
        return;
    }

    let mut vertex_buffers = CachedBindingArray::new();
    let mut index_buffers = CachedBindingArray::new();
    let mut textures = CachedBindingArray::new();
    let mut samplers = Vec::new();
    let mut tlas = TlasPackage::new(render_device.wgpu_device().create_tlas(
        &CreateTlasDescriptor {
            label: Some("tlas"),
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: AccelerationStructureUpdateMode::Build,
            max_instances: instances.iter().len() as u32,
        },
    ));
    let mut transforms = StorageBuffer::<Vec<Mat4>>::default();
    let mut geometry_ids = StorageBuffer::<Vec<UVec4>>::default();
    let mut material_ids = StorageBuffer::<Vec<u32>>::default();
    let mut materials = StorageBuffer::<Vec<GpuRaytracingMaterial>>::default();

    let mut material_id_map: HashMap<AssetId<StandardMaterial>, u32, FixedHasher> =
        HashMap::default();
    let mut material_id = 0;
    let mut process_texture = |texture_handle: &Option<Handle<_>>| -> Option<u32> {
        match texture_handle {
            Some(texture_handle) => match texture_assets.get(texture_handle.id()) {
                Some(texture) => {
                    let (texture_id, is_new) =
                        textures.push_if_absent(texture.texture_view.deref(), texture_handle.id());
                    if is_new {
                        samplers.push(texture.sampler.deref());
                    }
                    Some(texture_id)
                }
                None => None,
            },
            None => Some(u32::MAX),
        }
    };
    for (asset_id, material) in material_assets.iter() {
        let Some(base_color_texture_id) = process_texture(&material.base_color_texture) else {
            continue;
        };
        let Some(normal_map_texture_id) = process_texture(&material.normal_map_texture) else {
            continue;
        };
        let Some(emissive_texture_id) = process_texture(&material.emissive_texture) else {
            continue;
        };

        materials.get_mut().push(GpuRaytracingMaterial {
            base_color: material.base_color.to_linear(),
            emissive: material.emissive,
            base_color_texture_id,
            normal_map_texture_id,
            emissive_texture_id,
            _padding: Default::default(),
        });

        material_id_map.insert(*asset_id, material_id);
        material_id += 1;
    }

    if material_id == 0 {
        return;
    }

    if textures.is_empty() {
        textures.vec.push(fallback_texture.d2.texture_view.deref());
        samplers.push(fallback_texture.d2.sampler.deref());
    }

    let mut instance_id = 0;
    for (mesh, material, transform) in &instances {
        let Some(blas) = blas_manager.get(&mesh.id()) else {
            continue;
        };
        let Some(vertex_slice) = mesh_allocator.mesh_vertex_slice(&mesh.id()) else {
            continue;
        };
        let Some(index_slice) = mesh_allocator.mesh_index_slice(&mesh.id()) else {
            continue;
        };
        let Some(material_id) = material_id_map.get(&material.id()) else {
            continue;
        };

        let transform = transform.compute_matrix();
        *tlas.get_mut_single(instance_id).unwrap() = Some(TlasInstance::new(
            blas,
            tlas_transform(&transform),
            instance_id as u32,
            0xFF,
        ));

        transforms.get_mut().push(transform);

        let (vertex_buffer_id, _) = vertex_buffers.push_if_absent(
            vertex_slice.buffer.as_entire_buffer_binding(),
            vertex_slice.buffer.id(),
        );
        let (index_buffer_id, _) = index_buffers.push_if_absent(
            index_slice.buffer.as_entire_buffer_binding(),
            index_slice.buffer.id(),
        );

        geometry_ids.get_mut().push(UVec4::new(
            vertex_buffer_id,
            vertex_slice.range.start,
            index_buffer_id,
            index_slice.range.start,
        ));

        material_ids.get_mut().push(*material_id);

        instance_id += 1;
    }

    if instance_id == 0 {
        return;
    }

    transforms.write_buffer(&render_device, &render_queue);
    geometry_ids.write_buffer(&render_device, &render_queue);
    material_ids.write_buffer(&render_device, &render_queue);
    materials.write_buffer(&render_device, &render_queue);

    let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("build_tlas_command_encoder"),
    });
    command_encoder.build_acceleration_structures(&[], [&tlas]);
    render_queue.submit([command_encoder.finish()]);

    raytracing_scene_bindings.bind_group = Some(render_device.create_bind_group(
        "raytracing_scene_bind_group",
        &raytracing_scene_bindings.bind_group_layout,
        &BindGroupEntries::sequential((
            vertex_buffers.as_slice(),
            index_buffers.as_slice(),
            textures.as_slice(),
            samplers.as_slice(),
            tlas.as_binding(),
            transforms.binding().unwrap(),
            geometry_ids.binding().unwrap(),
            material_ids.binding().unwrap(),
            materials.binding().unwrap(),
        )),
    ));
}

impl FromWorld for RaytracingSceneBindings {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout_entries = &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: MAX_MESH_COUNT,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: MAX_MESH_COUNT,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: MAX_TEXTURE_COUNT,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: MAX_TEXTURE_COUNT,
            },
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::AccelerationStructure,
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
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
        ];

        Self {
            bind_group: None,
            bind_group_layout: render_device.create_bind_group_layout(
                "raytracing_scene_bind_group_layout",
                bind_group_layout_entries,
            ),
        }
    }
}

struct CachedBindingArray<T, I: Eq + Hash> {
    map: HashMap<I, u32>,
    vec: Vec<T>,
}

impl<T, I: Eq + Hash> CachedBindingArray<T, I> {
    fn new() -> Self {
        Self {
            map: HashMap::default(),
            vec: Vec::default(),
        }
    }

    fn push_if_absent(&mut self, item: T, item_id: I) -> (u32, bool) {
        let mut is_new = false;
        let i = *self.map.entry(item_id).or_insert_with(|| {
            is_new = true;
            let i = self.vec.len() as u32;
            self.vec.push(item);
            i
        });
        (i, is_new)
    }

    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    fn as_slice(&self) -> &[T] {
        self.vec.as_slice()
    }
}

#[derive(ShaderType)]
struct GpuRaytracingMaterial {
    base_color: LinearRgba,
    emissive: LinearRgba,
    base_color_texture_id: u32,
    normal_map_texture_id: u32,
    emissive_texture_id: u32,
    _padding: u32,
}

fn tlas_transform(transform: &Mat4) -> [f32; 12] {
    transform.transpose().to_cols_array()[..12]
        .try_into()
        .unwrap()
}
