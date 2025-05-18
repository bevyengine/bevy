use super::{blas::BlasManager, extract::StandardMaterialAssets, RaytracingMesh3d};
use bevy_asset::{AssetId, Handle};
use bevy_color::LinearRgba;
use bevy_ecs::{
    resource::Resource,
    system::{Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::{Mat4, Vec3};
use bevy_pbr::{ExtractedDirectionalLight, MeshMaterial3d, StandardMaterial};
use bevy_platform::{collections::HashMap, hash::FixedHasher};
use bevy_render::{
    mesh::allocator::MeshAllocator,
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{FallbackImage, GpuImage},
};
use bevy_transform::components::GlobalTransform;
use std::{hash::Hash, num::NonZeroU32, ops::Deref};

const MAX_MESH_SLAB_COUNT: Option<NonZeroU32> = NonZeroU32::new(500);
const MAX_TEXTURE_COUNT: Option<NonZeroU32> = NonZeroU32::new(5_000);

/// Average angular diameter of the sun as seen from earth.
/// https://en.wikipedia.org/wiki/Angular_diameter#Use_in_astronomy
const SUN_ANGULAR_DIAMETER_RADIANS: f32 = 0.00930842;

#[derive(Resource)]
pub struct RaytracingSceneBindings {
    pub bind_group: Option<BindGroup>,
    pub bind_group_layout: BindGroupLayout,
}

pub fn prepare_raytracing_scene_bindings(
    instances_query: Query<(
        &RaytracingMesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &GlobalTransform,
    )>,
    directional_lights_query: Query<&ExtractedDirectionalLight>,
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

    if instances_query.iter().len() == 0 {
        return;
    }

    let mut vertex_buffers = CachedBindingArray::new();
    let mut index_buffers = CachedBindingArray::new();
    let mut textures = CachedBindingArray::new();
    let mut samplers = Vec::new();
    let mut materials = StorageBufferList::<GpuMaterial>::default();
    let mut tlas = TlasPackage::new(render_device.wgpu_device().create_tlas(
        &CreateTlasDescriptor {
            label: Some("tlas"),
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: AccelerationStructureUpdateMode::Build,
            max_instances: instances_query.iter().len() as u32,
        },
    ));
    let mut transforms = StorageBufferList::<Mat4>::default();
    let mut geometry_ids = StorageBufferList::<GpuInstanceGeometryIds>::default();
    let mut material_ids = StorageBufferList::<u32>::default();
    let mut light_sources = StorageBufferList::<GpuLightSource>::default();
    let mut directional_lights = StorageBufferList::<GpuDirectionalLight>::default();

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

        materials.get_mut().push(GpuMaterial {
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
    for (mesh, material, transform) in &instances_query {
        let Some(blas) = blas_manager.get(&mesh.id()) else {
            continue;
        };
        let Some(vertex_slice) = mesh_allocator.mesh_vertex_slice(&mesh.id()) else {
            continue;
        };
        let Some(index_slice) = mesh_allocator.mesh_index_slice(&mesh.id()) else {
            continue;
        };
        let Some(material_id) = material_id_map.get(&material.id()).copied() else {
            continue;
        };
        let Some(material) = materials.get().get(material_id as usize) else {
            continue;
        };

        let transform = transform.compute_matrix();
        *tlas.get_mut_single(instance_id).unwrap() = Some(TlasInstance::new(
            blas,
            tlas_transform(&transform),
            Default::default(),
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

        geometry_ids.get_mut().push(GpuInstanceGeometryIds {
            vertex_buffer_id,
            vertex_buffer_offset: vertex_slice.range.start,
            index_buffer_id,
            index_buffer_offset: index_slice.range.start,
        });

        material_ids.get_mut().push(material_id);

        if material.emissive != LinearRgba::BLACK {
            light_sources
                .get_mut()
                .push(GpuLightSource::new_emissive_mesh_light(
                    instance_id as u32,
                    (index_slice.range.len() / 3) as u32,
                ));
        }

        instance_id += 1;
    }

    if instance_id == 0 {
        return;
    }

    for directional_light in &directional_lights_query {
        let directional_lights = directional_lights.get_mut();
        let directional_light_id = directional_lights.len() as u32;

        directional_lights.push(GpuDirectionalLight {
            direction_to_light: directional_light.transform.back().into(),
            cos_theta_max: (SUN_ANGULAR_DIAMETER_RADIANS / 2.0).cos(),
            illuminance: directional_light.color * directional_light.illuminance,
        });

        light_sources
            .get_mut()
            .push(GpuLightSource::new_directional_light(directional_light_id));
    }

    materials.write_buffer(&render_device, &render_queue);
    transforms.write_buffer(&render_device, &render_queue);
    geometry_ids.write_buffer(&render_device, &render_queue);
    material_ids.write_buffer(&render_device, &render_queue);
    light_sources.write_buffer(&render_device, &render_queue);
    directional_lights.write_buffer(&render_device, &render_queue);

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
            materials.binding().unwrap(),
            tlas.as_binding(),
            transforms.binding().unwrap(),
            geometry_ids.binding().unwrap(),
            material_ids.binding().unwrap(),
            light_sources.binding().unwrap(),
            directional_lights.binding().unwrap(),
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
                count: MAX_MESH_SLAB_COUNT,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: MAX_MESH_SLAB_COUNT,
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
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::AccelerationStructure,
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
            BindGroupLayoutEntry {
                binding: 9,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 10,
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

type StorageBufferList<T> = StorageBuffer<Vec<T>>;

#[derive(ShaderType)]
struct GpuInstanceGeometryIds {
    vertex_buffer_id: u32,
    vertex_buffer_offset: u32,
    index_buffer_id: u32,
    index_buffer_offset: u32,
}

#[derive(ShaderType)]
struct GpuMaterial {
    base_color: LinearRgba,
    emissive: LinearRgba,
    base_color_texture_id: u32,
    normal_map_texture_id: u32,
    emissive_texture_id: u32,
    _padding: u32,
}

#[derive(ShaderType)]
struct GpuLightSource {
    kind: u32,
    id: u32,
}

impl GpuLightSource {
    fn new_emissive_mesh_light(instance_id: u32, triangle_count: u32) -> GpuLightSource {
        Self {
            kind: triangle_count << 1,
            id: instance_id,
        }
    }

    fn new_directional_light(directional_light_id: u32) -> GpuLightSource {
        Self {
            kind: 1,
            id: directional_light_id,
        }
    }
}

#[derive(ShaderType, Default)]
struct GpuDirectionalLight {
    direction_to_light: Vec3,
    cos_theta_max: f32,
    illuminance: LinearRgba,
}

fn tlas_transform(transform: &Mat4) -> [f32; 12] {
    transform.transpose().to_cols_array()[..12]
        .try_into()
        .unwrap()
}
