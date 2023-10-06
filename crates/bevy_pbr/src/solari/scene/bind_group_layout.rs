use super::scene_types::{GpuSolariMaterial, SolariUniforms};
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_math::Mat4;
use bevy_render::{render_resource::*, renderer::RenderDevice};
use std::num::NonZeroU32;

#[derive(Resource)]
pub struct SolariSceneBindGroupLayout(pub BindGroupLayout);

impl FromWorld for SolariSceneBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let entries = &[
            // TLAS
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::AccelerationStructure,
                count: None,
            },
            // Mesh material indices buffer
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(u32::min_size()),
                },
                count: None,
            },
            // Index buffers
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None, // TODO
                },
                count: Some(unsafe { NonZeroU32::new_unchecked(10_000) }),
            },
            // Vertex buffers
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None, // TODO
                },
                count: Some(unsafe { NonZeroU32::new_unchecked(10_000) }),
            },
            // Transforms buffer
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(Mat4::min_size()),
                },
                count: None,
            },
            // Material buffer
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(GpuSolariMaterial::min_size()),
                },
                count: None,
            },
            // Texture maps
            BindGroupLayoutEntry {
                binding: 6,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: Some(unsafe { NonZeroU32::new_unchecked(10_000) }),
            },
            // Texture map samplers
            BindGroupLayoutEntry {
                binding: 7,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: Some(unsafe { NonZeroU32::new_unchecked(10_000) }),
            },
            // Emissive object mesh material indices buffer
            BindGroupLayoutEntry {
                binding: 8,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(u32::min_size()),
                },
                count: None,
            },
            // Emissive object triangle counts buffer
            BindGroupLayoutEntry {
                binding: 9,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(u32::min_size()),
                },
                count: None,
            },
            // Uniforms
            BindGroupLayoutEntry {
                binding: 10,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(SolariUniforms::min_size()),
                },
                count: None,
            },
        ];

        Self(
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("solari_scene_bind_group_layout"),
                entries,
            }),
        )
    }
}
