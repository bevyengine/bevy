use super::RaytracingMesh3d;
use bevy_ecs::{
    resource::Resource,
    system::{Query, Res},
    world::{FromWorld, World},
};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_render::{
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
};
use std::num::NonZeroU32;

const MAX_MESH_COUNT: Option<NonZeroU32> = NonZeroU32::new(2u32.pow(16));
const MAX_TEXTURE_COUNT: Option<NonZeroU32> = NonZeroU32::new(10_000);

#[derive(Resource)]
pub struct RaytracingSceneBindings {
    bind_group: Option<BindGroup>,
    bind_group_layout: BindGroupLayout,
}

pub fn prepare_raytracing_scene_bindings(
    instances: Query<(&RaytracingMesh3d, &MeshMaterial3d<StandardMaterial>)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
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
