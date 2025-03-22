use bevy_asset::{AssetId, Handle};
use bevy_core_pipeline::{
    core_2d::CORE_2D_DEPTH_FORMAT, tonemapping::get_lut_bind_group_layout_entries,
};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_image::{BevyDefault, Image, ImageSampler, TextureFormatPixelInfo};
use bevy_render::{
    batching::{gpu_preprocessing::IndirectParametersCpuMetadata, GetBatchData, GetFullBatchData},
    globals::GlobalsUniform,
    mesh::{allocator::MeshAllocator, Mesh, MeshVertexBufferLayoutRef, RenderMesh},
    render_asset::RenderAssets,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    sync_world::MainEntity,
    texture::{DefaultImageSampler, GpuImage},
    view::{ViewTarget, ViewUniform},
};

use nonmax::NonMaxU32;
use tracing::error;

use crate::material::rendering::Material2dBindGroupId;

use super::{instancing::RenderMesh2dInstances, shader_types::Mesh2dUniform};

/// Pipeline for rendering 2d meshes
#[derive(Resource, Clone)]
pub struct Mesh2dPipeline {
    /// [`BindGroupLayout`] of the view
    pub view_layout: BindGroupLayout,
    /// [`BindGroupLayout`] of the mesh
    pub mesh_layout: BindGroupLayout,
    /// Fallback image used for optional textures
    pub dummy_white_gpu_image: GpuImage,
    /// Size of the batch
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
    /// Gets [`TextureView`] and [`Sampler`] of an [`Image`] if it exists on the GPU.
    ///
    /// Optional textures will use the [`TextureView`] and [`Sampler`] of the fallback image.
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
        error!(
            "`get_index_and_compare_data` is only intended for GPU mesh uniform building, \
            but this is not yet implemented for 2d meshes"
        );
        None
    }

    fn get_binned_index(
        _: &SystemParamItem<Self::Param>,
        _query_item: MainEntity,
    ) -> Option<NonMaxU32> {
        error!(
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

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    // FIXME: make normals optional?
    /// Pipeline key for the [`Mesh2dPipeline`].
    pub struct Mesh2dPipelineKey: u32 {
        /// No optional feature is used by pipeline
        const NONE                              = 0;
        /// Pipeline uses HDR
        const HDR                               = 1 << 0;
        /// Pipeline performs tonemapping in shader
        const TONEMAP_IN_SHADER                 = 1 << 1;
        /// Pipeline performs debanding dither
        const DEBAND_DITHER                     = 1 << 2;
        /// Pipeline allows alpha blend
        const BLEND_ALPHA                       = 1 << 3;
        /// Pipeline allows discarding of fragments
        const MAY_DISCARD                       = 1 << 4;
        /// Mask for MSAA samples of pipeline
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        /// Mask for primitive topology of pipeline
        const PRIMITIVE_TOPOLOGY_RESERVED_BITS  = Self::PRIMITIVE_TOPOLOGY_MASK_BITS << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        /// Mask for tonemapping method of pipeline
        const TONEMAP_METHOD_RESERVED_BITS      = Self::TONEMAP_METHOD_MASK_BITS << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Tonemapping is disable on pipeline
        const TONEMAP_METHOD_NONE               = 0 << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Pipeline uses Reinhard tonemapping
        const TONEMAP_METHOD_REINHARD           = 1 << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Pipeline uses Reinhard Luminace tonemapping
        const TONEMAP_METHOD_REINHARD_LUMINANCE = 2 << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Pipeline uses AcesFitted tonemapping
        const TONEMAP_METHOD_ACES_FITTED        = 3 << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Pipeline uses Agx tonemapping
        const TONEMAP_METHOD_AGX                = 4 << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Pipeline uses Somewhat Boring Display Transform tonemapping
        const TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM = 5 << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Pipeline uses Tony McMapface tonemapping
        const TONEMAP_METHOD_TONY_MC_MAPFACE    = 6 << Self::TONEMAP_METHOD_SHIFT_BITS;
        /// Pipeline uses Blender Filmic tonemapping
        const TONEMAP_METHOD_BLENDER_FILMIC     = 7 << Self::TONEMAP_METHOD_SHIFT_BITS;
    }
}

impl Mesh2dPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();
    const PRIMITIVE_TOPOLOGY_MASK_BITS: u32 = 0b111;
    const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u32 = Self::MSAA_SHIFT_BITS - 3;
    const TONEMAP_METHOD_MASK_BITS: u32 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u32 =
        Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS - Self::TONEMAP_METHOD_MASK_BITS.count_ones();

    /// Creates a [`Mesh2dPipelineKey`] from the number of MSAA samples
    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    /// Gets the number of MSAA samples of [`Mesh2dPipelineKey`]
    pub fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    /// Creates a [`Mesh2dPipelineKey`] with HDR enabled or not
    pub fn from_hdr(hdr: bool) -> Self {
        if hdr {
            Mesh2dPipelineKey::HDR
        } else {
            Mesh2dPipelineKey::NONE
        }
    }

    /// Creates a [`Mesh2dPipelineKey`] from type of [`PrimitiveTopology`]
    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u32)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits)
    }

    /// Gets [`PrimitiveTopology`] of [`Mesh2dPipelineKey`]
    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits = (self.bits() >> Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u32 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u32 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u32 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u32 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u32 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
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
                shader: super::MESH2D_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: super::MESH2D_SHADER_HANDLE,
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
