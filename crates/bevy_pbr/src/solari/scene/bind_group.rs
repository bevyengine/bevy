use super::{
    bind_group_layout::SolariSceneBindGroupLayout,
    blas::BlasStorage,
    helpers::{new_storage_buffer, pack_object_indices, tlas_transform, IndexedVec},
    scene_types::{GpuSolariMaterial, SolariMaterial, SolariUniforms},
};
use crate::{ExtractedDirectionalLight, StandardMaterial};
use bevy_asset::Handle;
use bevy_core::FrameCount;
use bevy_ecs::system::{Query, Res, ResMut, Resource};
use bevy_render::{
    mesh::GpuBufferInfo,
    prelude::{Color, Mesh},
    render_asset::RenderAssets,
    render_resource::{raytrace::*, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{FallbackImage, Image},
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::default;
use std::iter;

#[derive(Resource, Default)]
pub struct SolariSceneBindGroup(pub Option<BindGroup>);

pub fn queue_scene_bind_group(
    objects: Query<(
        &Handle<Mesh>,
        &Handle<StandardMaterial>,
        &SolariMaterial,
        &GlobalTransform,
    )>,
    sun: Query<&ExtractedDirectionalLight>,
    mut scene_bind_group: ResMut<SolariSceneBindGroup>,
    scene_bind_group_layout: Res<SolariSceneBindGroupLayout>,
    mesh_assets: Res<RenderAssets<Mesh>>,
    image_assets: Res<RenderAssets<Image>>,
    blas_storage: Res<BlasStorage>,
    fallback_image: Res<FallbackImage>,
    frame_count: Res<FrameCount>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Create CPU buffers for scene resources
    // TODO: Reuse memory each frame via Local<T>
    let mut mesh_material_indices = Vec::new();
    let mut index_buffers = IndexedVec::new();
    let mut vertex_buffers = Vec::new();
    let mut transforms = Vec::new();
    let mut materials = IndexedVec::new();
    let mut texture_maps = IndexedVec::new();
    let mut samplers = Vec::new();
    let mut emissive_object_indices = Vec::new();
    let mut emissive_object_triangle_counts = Vec::new();
    let objects = objects.iter().collect::<Vec<_>>();

    let mut get_mesh_index = |mesh_handle: &Handle<Mesh>| {
        index_buffers.get_index(mesh_handle.clone_weak(), |mesh_handle| {
            let gpu_mesh = mesh_assets.get(&mesh_handle).unwrap();
            vertex_buffers.push(gpu_mesh.vertex_buffer.as_entire_buffer_binding());
            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed { buffer, .. } => buffer.as_entire_buffer_binding(),
                _ => unreachable!(),
            }
        })
    };

    let mut get_texture_map_index = |maybe_texture_map_handle: &Option<Handle<Image>>| {
        if let Some(texture_map_handle) = maybe_texture_map_handle.clone() {
            texture_maps.get_index(texture_map_handle, |texture_map_handle| {
                // TODO: Handle unwrap
                let image = image_assets.get(&texture_map_handle).unwrap();
                samplers.push(&*image.sampler);
                &*image.texture_view
            })
        } else {
            u32::MAX
        }
    };

    let mut get_material_index = |material_handle: &Handle<StandardMaterial>,
                                  material: &SolariMaterial| {
        let emissive = material.emissive.as_linear_rgba_f32();
        materials.get_index(material_handle.clone_weak(), |_| GpuSolariMaterial {
            base_color: material.base_color.as_linear_rgba_f32().into(),
            base_color_texture_index: get_texture_map_index(&material.base_color_texture),
            normal_map_texture_index: get_texture_map_index(&material.normal_map_texture),
            emissive: [emissive[0], emissive[1], emissive[2]].into(),
            emissive_texture_index: get_texture_map_index(&material.emissive_texture),
        })
    };

    // Create TLAS
    let mut tlas = TlasPackage::new(
        render_device
            .wgpu_device()
            .create_tlas(&CreateTlasDescriptor {
                label: Some("tlas"),
                flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: AccelerationStructureUpdateMode::Build,
                max_instances: objects.len() as u32,
            }),
        objects.len() as u32,
    );

    // Fill TLAS and scene buffers
    // TODO: Parallelize loop
    let mut object_i = 0;
    for (mesh_handle, material_handle, material, transform) in objects {
        if let Some(blas) = blas_storage.get(mesh_handle) {
            let mesh_index = get_mesh_index(mesh_handle);
            let material_index = get_material_index(material_handle, material);
            mesh_material_indices.push(pack_object_indices(mesh_index, material_index));

            let transform = transform.compute_matrix();
            transforms.push(transform);

            if material.emissive.as_rgba() != Color::BLACK || material.emissive_texture.is_some() {
                emissive_object_indices.push(object_i as u32);
                emissive_object_triangle_counts.push(
                    match mesh_assets.get(mesh_handle).unwrap().buffer_info {
                        GpuBufferInfo::Indexed { count, .. } => count / 3,
                        _ => unreachable!(),
                    },
                );
            }

            *tlas.get_mut_single(object_i).unwrap() = Some(TlasInstance::new(
                blas,
                tlas_transform(&transform),
                object_i as u32, // TODO: Max 24 bits
                0xFF,
            ));

            object_i += 1;
        }
    }

    // Build TLAS
    let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("build_tlas_command_encoder"),
    });
    command_encoder.build_acceleration_structures(&[], iter::once(&tlas));
    render_queue.submit([command_encoder.finish()]);

    // Upload buffers to the GPU
    // TODO: Reuse GPU buffers each frame
    let mesh_material_indices_buffer = new_storage_buffer(
        mesh_material_indices,
        "solari_mesh_material_indices_buffer",
        &render_device,
        &render_queue,
    );
    let transforms_buffer = new_storage_buffer(
        transforms,
        "solari_transforms_buffer",
        &render_device,
        &render_queue,
    );
    let materials_buffer = new_storage_buffer(
        materials.vec,
        "solari_material_buffer",
        &render_device,
        &render_queue,
    );
    let emissive_object_indices_buffer = new_storage_buffer(
        emissive_object_indices,
        "solari_emissive_object_indices_buffer",
        &render_device,
        &render_queue,
    );
    let emissive_object_triangle_counts_buffer = new_storage_buffer(
        emissive_object_triangle_counts,
        "solari_emissive_object_triangle_counts_buffer",
        &render_device,
        &render_queue,
    );

    // Ensure binding arrays are non-empty
    if vertex_buffers.is_empty() {
        scene_bind_group.0 = None;
        return;
    }
    if texture_maps.vec.is_empty() {
        texture_maps.vec.push(&fallback_image.d2.texture_view);
    }
    if samplers.is_empty() {
        samplers.push(&fallback_image.d2.sampler);
    }

    // Build uniforms
    // TODO: Handle multiple directional lights
    let dummy_sun = &ExtractedDirectionalLight {
        color: Color::BLACK,
        illuminance: 0.0,
        ..default()
    };
    let sun = sun.get_single().unwrap_or(dummy_sun);
    let uniforms = &SolariUniforms::new(&frame_count, sun, &render_device, &render_queue);

    // Create scene bind group
    scene_bind_group.0 = Some(render_device.create_bind_group(&BindGroupDescriptor {
        label: Some("solari_scene_bind_group"),
        layout: &scene_bind_group_layout.0,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::AccelerationStructure(tlas.tlas()),
            },
            BindGroupEntry {
                binding: 1,
                resource: mesh_material_indices_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::BufferArray(index_buffers.vec.as_slice()),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::BufferArray(vertex_buffers.as_slice()),
            },
            BindGroupEntry {
                binding: 4,
                resource: transforms_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 5,
                resource: materials_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 6,
                resource: BindingResource::TextureViewArray(texture_maps.vec.as_slice()),
            },
            BindGroupEntry {
                binding: 7,
                resource: BindingResource::SamplerArray(&samplers),
            },
            BindGroupEntry {
                binding: 8,
                resource: emissive_object_indices_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 9,
                resource: emissive_object_triangle_counts_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 10,
                resource: uniforms.binding().unwrap(),
            },
        ],
    }));
}
