use bevy_asset::{AssetId, Handle};
use bevy_core_pipeline::{
    core_2d::CORE_2D_DEPTH_FORMAT, tonemapping::get_lut_bind_group_layout_entries,
};
use bevy_ecs::{
    entity::Entity,
    resource::Resource,
    system::{lifetimeless::SRes, Res, SystemParamItem, SystemState},
    world::{FromWorld, World},
};
use bevy_image::{BevyDefault, Image, ImageSampler, TextureFormatPixelInfo};
use bevy_render::{
    batching::{gpu_preprocessing::IndirectParametersCpuMetadata, GetBatchData, GetFullBatchData},
    globals::GlobalsUniform,
    mesh::{allocator::MeshAllocator, Mesh, MeshVertexBufferLayoutRef, RenderMesh},
    render_asset::RenderAssets,
    render_resource::{
        binding_types::uniform_buffer, BindGroupLayout, BindGroupLayoutEntries, BlendState,
        ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState,
        FragmentState, FrontFace, GpuArrayBuffer, MultisampleState, PolygonMode, PrimitiveState,
        RenderPipelineDescriptor, Sampler, ShaderDefVal, ShaderStages, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, StencilFaceState, StencilState, TexelCopyBufferLayout,
        TextureFormat, TextureView, TextureViewDescriptor, VertexState,
    },
    renderer::{RenderDevice, RenderQueue},
    sync_world::MainEntity,
    texture::{DefaultImageSampler, GpuImage},
    view::{ViewTarget, ViewUniform},
};

use nonmax::NonMaxU32;

use super::{
    key::Mesh2dPipelineKey,
    render::{Material2dBindGroupId, Mesh2dUniform, RenderMesh2dInstances},
    MESH2D_SHADER_HANDLE,
};

#[derive(Resource, Clone)]
pub struct Mesh2dPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    // This dummy white texture is to be used in place of optional textures
    pub dummy_white_gpu_image: GpuImage,
    pub per_object_buffer_batch_size: Option<u32>,
}

impl FromWorld for Mesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<RenderQueue>,
            Res<DefaultImageSampler>,
        )> = SystemState::new(world);
        let (render_device, render_queue, default_sampler) = system_state.get_mut(world);
        let render_device = render_device.into_inner();
        let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
        let view_layout = render_device.create_bind_group_layout(
            "mesh2d_view_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    (0, uniform_buffer::<ViewUniform>(true)),
                    (1, uniform_buffer::<GlobalsUniform>(false)),
                    (
                        2,
                        tonemapping_lut_entries[0].visibility(ShaderStages::FRAGMENT),
                    ),
                    (
                        3,
                        tonemapping_lut_entries[1].visibility(ShaderStages::FRAGMENT),
                    ),
                ),
            ),
        );

        let mesh_layout = render_device.create_bind_group_layout(
            "mesh2d_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                GpuArrayBuffer::<Mesh2dUniform>::binding_layout(render_device),
            ),
        );
        // A 1x1x1 'all 1.0' texture to use as a dummy texture to use in place of optional StandardMaterial textures
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                image.data.as_ref().expect("Image has no data"),
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );

            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.texture_descriptor.size,
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };
        Mesh2dPipeline {
            view_layout,
            mesh_layout,
            dummy_white_gpu_image,
            per_object_buffer_batch_size: GpuArrayBuffer::<Mesh2dUniform>::batch_size(
                render_device,
            ),
        }
    }
}

impl Mesh2dPipeline {
    pub fn get_image_texture<'a>(
        &'a self,
        gpu_images: &'a RenderAssets<GpuImage>,
        handle_option: &Option<Handle<Image>>,
    ) -> Option<(&'a TextureView, &'a Sampler)> {
        if let Some(handle) = handle_option {
            let gpu_image = gpu_images.get(handle)?;
            Some((&gpu_image.texture_view, &gpu_image.sampler))
        } else {
            Some((
                &self.dummy_white_gpu_image.texture_view,
                &self.dummy_white_gpu_image.sampler,
            ))
        }
    }
}

impl GetBatchData for Mesh2dPipeline {
    type Param = (
        SRes<RenderMesh2dInstances>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
    );
    type CompareData = (Material2dBindGroupId, AssetId<Mesh>);
    type BufferData = Mesh2dUniform;

    fn get_batch_data(
        (mesh_instances, _, _): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let mesh_instance = mesh_instances.get(&main_entity)?;
        Some((
            Mesh2dUniform::from_components(&mesh_instance.transforms, mesh_instance.tag),
            mesh_instance.automatic_batching.then_some((
                mesh_instance.material_bind_group_id,
                mesh_instance.mesh_asset_id,
            )),
        ))
    }
}

impl GetFullBatchData for Mesh2dPipeline {
    type BufferInputData = ();

    fn get_binned_batch_data(
        (mesh_instances, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<Self::BufferData> {
        let mesh_instance = mesh_instances.get(&main_entity)?;
        Some(Mesh2dUniform::from_components(
            &mesh_instance.transforms,
            mesh_instance.tag,
        ))
    }

    fn get_index_and_compare_data(
        _: &SystemParamItem<Self::Param>,
        _query_item: MainEntity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        tracing::error!(
            "`get_index_and_compare_data` is only intended for GPU mesh uniform building, \
            but this is not yet implemented for 2d meshes"
        );
        None
    }

    fn get_binned_index(
        _: &SystemParamItem<Self::Param>,
        _query_item: MainEntity,
    ) -> Option<NonMaxU32> {
        tracing::error!(
            "`get_binned_index` is only intended for GPU mesh uniform building, \
            but this is not yet implemented for 2d meshes"
        );
        None
    }

    fn write_batch_indirect_parameters_metadata(
        indexed: bool,
        base_output_index: u32,
        batch_set_index: Option<NonMaxU32>,
        indirect_parameters_buffer: &mut bevy_render::batching::gpu_preprocessing::UntypedPhaseIndirectParametersBuffers,
        indirect_parameters_offset: u32,
    ) {
        // Note that `IndirectParameters` covers both of these structures, even
        // though they actually have distinct layouts. See the comment above that
        // type for more information.
        let indirect_parameters = IndirectParametersCpuMetadata {
            base_output_index,
            batch_set_index: match batch_set_index {
                None => !0,
                Some(batch_set_index) => u32::from(batch_set_index),
            },
        };

        if indexed {
            indirect_parameters_buffer
                .indexed
                .set(indirect_parameters_offset, indirect_parameters);
        } else {
            indirect_parameters_buffer
                .non_indexed
                .set(indirect_parameters_offset, indirect_parameters);
        }
    }
}

impl SpecializedMeshPipeline for Mesh2dPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_NORMAL) {
            shader_defs.push("VERTEX_NORMALS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(1));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push("VERTEX_TANGENTS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(3));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(4));
        }

        if key.contains(Mesh2dPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                2,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                3,
            ));

            let method = key.intersection(Mesh2dPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            match method {
                Mesh2dPipelineKey::TONEMAP_METHOD_NONE => {
                    shader_defs.push("TONEMAP_METHOD_NONE".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD => {
                    shader_defs.push("TONEMAP_METHOD_REINHARD".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE => {
                    shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_ACES_FITTED => {
                    shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_AGX => {
                    shader_defs.push("TONEMAP_METHOD_AGX".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM => {
                    shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC => {
                    shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE => {
                    shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
                }
                _ => {}
            }
            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(Mesh2dPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if key.contains(Mesh2dPipelineKey::MAY_DISCARD) {
            shader_defs.push("MAY_DISCARD".into());
        }

        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        let format = match key.contains(Mesh2dPipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        let (depth_write_enabled, label, blend);
        if key.contains(Mesh2dPipelineKey::BLEND_ALPHA) {
            label = "transparent_mesh2d_pipeline";
            blend = Some(BlendState::ALPHA_BLENDING);
            depth_write_enabled = false;
        } else {
            label = "opaque_mesh2d_pipeline";
            blend = None;
            depth_write_enabled = true;
        }

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: MESH2D_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: MESH2D_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.mesh_layout.clone()],
            push_constant_ranges: vec![],
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_2D_DEPTH_FORMAT,
                depth_write_enabled,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some(label.into()),
            zero_initialize_workgroup_memory: false,
        })
    }
}
