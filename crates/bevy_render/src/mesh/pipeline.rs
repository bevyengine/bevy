use bevy_asset::AssetId;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_image::BevyDefault;
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef, VertexAttributeDescriptor};
use bevy_render::{
    batching::{
        gpu_preprocessing::{IndirectParametersCpuMetadata, UntypedPhaseIndirectParametersBuffers},
        GetBatchData, GetFullBatchData,
    },
    mesh::{allocator::MeshAllocator, RenderMesh},
    render_asset::RenderAssets,
    render_resource::*,
    view::ViewTarget,
};
use bevy_shader::ShaderDefVal;
use bevy_utils::default;
use tracing::error;

use bevy_render::sync_world::MainEntity;

pub use bevy_material::render::*;
use nonmax::NonMaxU32;

use crate::{
    lightmap::{LightmapSlabIndex, RenderLightmaps},
    mesh::{
        material_bind_group::MaterialBindGroupIndex,
        render::{MeshInputUniform, MeshUniform, RenderMeshInstances},
        skin::SkinUniforms,
        util::{
            CORE_3D_DEPTH_FORMAT, IRRADIANCE_VOLUMES_ARE_USABLE,
            TONEMAPPING_LUT_SAMPLER_BINDING_INDEX, TONEMAPPING_LUT_TEXTURE_BINDING_INDEX,
        },
    },
};

impl GetBatchData for MeshPipeline {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<RenderLightmaps>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
        SRes<SkinUniforms>,
    );
    // The material bind group ID, the mesh ID, and the lightmap ID,
    // respectively.
    type CompareData = (
        MaterialBindGroupIndex,
        AssetId<Mesh>,
        Option<LightmapSlabIndex>,
    );

    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, lightmaps, _, mesh_allocator, skin_uniforms): &SystemParamItem<
            Self::Param,
        >,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };
        let maybe_lightmap = lightmaps.render_lightmaps.get(&main_entity);

        let current_skin_index = skin_uniforms.skin_index(main_entity);
        let material_bind_group_index = mesh_instance.material_bindings_index;

        Some((
            MeshUniform::new(
                &mesh_instance.transforms,
                first_vertex_index,
                material_bind_group_index.slot,
                maybe_lightmap.map(|lightmap| (lightmap.slot_index, lightmap.uv_rect)),
                current_skin_index,
                Some(mesh_instance.tag),
            ),
            mesh_instance.should_batch().then_some((
                material_bind_group_index.group,
                mesh_instance.mesh_asset_id,
                maybe_lightmap.map(|lightmap| lightmap.slab_index),
            )),
        ))
    }
}

impl GetFullBatchData for MeshPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (mesh_instances, lightmaps, _, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_index_and_compare_data` should never be called in CPU mesh uniform building \
                mode"
            );
            return None;
        };

        let mesh_instance = mesh_instances.get(&main_entity)?;
        let maybe_lightmap = lightmaps.render_lightmaps.get(&main_entity);

        Some((
            mesh_instance.current_uniform_index,
            mesh_instance.should_batch().then_some((
                mesh_instance.material_bindings_index.group,
                mesh_instance.mesh_asset_id,
                maybe_lightmap.map(|lightmap| lightmap.slab_index),
            )),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, lightmaps, _, mesh_allocator, skin_uniforms): &SystemParamItem<
            Self::Param,
        >,
        main_entity: MainEntity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };
        let maybe_lightmap = lightmaps.render_lightmaps.get(&main_entity);

        let current_skin_index = skin_uniforms.skin_index(main_entity);

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            first_vertex_index,
            mesh_instance.material_bindings_index.slot,
            maybe_lightmap.map(|lightmap| (lightmap.slot_index, lightmap.uv_rect)),
            current_skin_index,
            Some(mesh_instance.tag),
        ))
    }

    fn get_binned_index(
        (mesh_instances, _, _, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<NonMaxU32> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_index` should never be called in CPU mesh uniform \
                building mode"
            );
            return None;
        };

        mesh_instances
            .get(&main_entity)
            .map(|entity| entity.current_uniform_index)
    }

    fn write_batch_indirect_parameters_metadata(
        indexed: bool,
        base_output_index: u32,
        batch_set_index: Option<NonMaxU32>,
        phase_indirect_parameters_buffers: &mut UntypedPhaseIndirectParametersBuffers,
        indirect_parameters_offset: u32,
    ) {
        let indirect_parameters = IndirectParametersCpuMetadata {
            base_output_index,
            batch_set_index: match batch_set_index {
                Some(batch_set_index) => u32::from(batch_set_index),
                None => !0,
            },
        };

        if indexed {
            phase_indirect_parameters_buffers
                .indexed
                .set(indirect_parameters_offset, indirect_parameters);
        } else {
            phase_indirect_parameters_buffers
                .non_indexed
                .set(indirect_parameters_offset, indirect_parameters);
        }
    }
}

impl SpecializedMeshPipeline for MeshPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        // Let the shader code know that it's running in a mesh pipeline.
        shader_defs.push("MESH_PIPELINE".into());

        shader_defs.push("VERTEX_OUTPUT_INSTANCE_INDEX".into());

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
            shader_defs.push("VERTEX_UVS_A".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_1) {
            shader_defs.push("VERTEX_UVS".into());
            shader_defs.push("VERTEX_UVS_B".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_1.at_shader_location(3));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push("VERTEX_TANGENTS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(4));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(5));
        }

        if cfg!(feature = "pbr_transmission_textures") {
            shader_defs.push("PBR_TRANSMISSION_TEXTURES_SUPPORTED".into());
        }
        if cfg!(feature = "pbr_multi_layer_material_textures") {
            shader_defs.push("PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED".into());
        }
        if cfg!(feature = "pbr_anisotropy_texture") {
            shader_defs.push("PBR_ANISOTROPY_TEXTURE_SUPPORTED".into());
        }
        if cfg!(feature = "pbr_specular_textures") {
            shader_defs.push("PBR_SPECULAR_TEXTURES_SUPPORTED".into());
        }

        let bind_group_layout = self.get_view_layout(key.into());
        let mut bind_group_layout = vec![
            bind_group_layout.main_layout.clone(),
            bind_group_layout.binding_array_layout.clone(),
        ];

        if key.msaa_samples() > 1 {
            shader_defs.push("MULTISAMPLED".into());
        };

        bind_group_layout.push(setup_morph_and_skinning_defs(
            &self.mesh_layouts,
            layout,
            6,
            &key,
            &mut shader_defs,
            &mut vertex_attributes,
            self.skins_use_uniform_buffers,
        ));

        if key.contains(MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION) {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        let (label, blend, depth_write_enabled);
        let pass = key.intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
        let (mut is_opaque, mut alpha_to_coverage_enabled) = (false, false);
        if key.contains(MeshPipelineKey::OIT_ENABLED) && pass == MeshPipelineKey::BLEND_ALPHA {
            label = "oit_mesh_pipeline".into();
            // TODO tail blending would need alpha blending
            blend = None;
            shader_defs.push("OIT_ENABLED".into());
            // TODO it should be possible to use this to combine MSAA and OIT
            // alpha_to_coverage_enabled = true;
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_ALPHA {
            label = "alpha_blend_mesh_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA {
            label = "premultiplied_alpha_mesh_pipeline".into();
            blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
            shader_defs.push("PREMULTIPLY_ALPHA".into());
            shader_defs.push("BLEND_PREMULTIPLIED_ALPHA".into());
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_MULTIPLY {
            label = "multiply_mesh_pipeline".into();
            blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent::OVER,
            });
            shader_defs.push("PREMULTIPLY_ALPHA".into());
            shader_defs.push("BLEND_MULTIPLY".into());
            // For the multiply pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else if pass == MeshPipelineKey::BLEND_ALPHA_TO_COVERAGE {
            label = "alpha_to_coverage_mesh_pipeline".into();
            // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases
            blend = None;
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
            is_opaque = !key.contains(MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE);
            alpha_to_coverage_enabled = true;
            shader_defs.push("ALPHA_TO_COVERAGE".into());
        } else {
            label = "opaque_mesh_pipeline".into();
            // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases
            blend = None;
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
            is_opaque = !key.contains(MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE);
        }

        if key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::HAS_PREVIOUS_SKIN) {
            shader_defs.push("HAS_PREVIOUS_SKIN".into());
        }

        if key.contains(MeshPipelineKey::HAS_PREVIOUS_MORPH) {
            shader_defs.push("HAS_PREVIOUS_MORPH".into());
        }

        if key.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            shader_defs.push("DEFERRED_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::NORMAL_PREPASS) && key.msaa_samples() == 1 && is_opaque {
            shader_defs.push("LOAD_PREPASS_NORMALS".into());
        }

        let view_projection = key.intersection(MeshPipelineKey::VIEW_PROJECTION_RESERVED_BITS);
        if view_projection == MeshPipelineKey::VIEW_PROJECTION_NONSTANDARD {
            shader_defs.push("VIEW_PROJECTION_NONSTANDARD".into());
        } else if view_projection == MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE {
            shader_defs.push("VIEW_PROJECTION_PERSPECTIVE".into());
        } else if view_projection == MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC {
            shader_defs.push("VIEW_PROJECTION_ORTHOGRAPHIC".into());
        }

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("WEBGL2".into());

        #[cfg(feature = "experimental_pbr_pcss")]
        shader_defs.push("PCSS_SAMPLERS_AVAILABLE".into());

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                TONEMAPPING_LUT_TEXTURE_BINDING_INDEX,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                TONEMAPPING_LUT_SAMPLER_BINDING_INDEX,
            ));

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(MeshPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if key.contains(MeshPipelineKey::MAY_DISCARD) {
            shader_defs.push("MAY_DISCARD".into());
        }

        if key.contains(MeshPipelineKey::ENVIRONMENT_MAP) {
            shader_defs.push("ENVIRONMENT_MAP".into());
        }

        if key.contains(MeshPipelineKey::IRRADIANCE_VOLUME) && IRRADIANCE_VOLUMES_ARE_USABLE {
            shader_defs.push("IRRADIANCE_VOLUME".into());
        }

        if key.contains(MeshPipelineKey::LIGHTMAPPED) {
            shader_defs.push("LIGHTMAP".into());
        }
        if key.contains(MeshPipelineKey::LIGHTMAP_BICUBIC_SAMPLING) {
            shader_defs.push("LIGHTMAP_BICUBIC_SAMPLING".into());
        }

        if key.contains(MeshPipelineKey::TEMPORAL_JITTER) {
            shader_defs.push("TEMPORAL_JITTER".into());
        }

        let shadow_filter_method =
            key.intersection(MeshPipelineKey::SHADOW_FILTER_METHOD_RESERVED_BITS);
        if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2 {
            shader_defs.push("SHADOW_FILTER_METHOD_HARDWARE_2X2".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_GAUSSIAN {
            shader_defs.push("SHADOW_FILTER_METHOD_GAUSSIAN".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_TEMPORAL {
            shader_defs.push("SHADOW_FILTER_METHOD_TEMPORAL".into());
        }

        let blur_quality =
            key.intersection(MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_RESERVED_BITS);

        shader_defs.push(ShaderDefVal::Int(
            "SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS".into(),
            match blur_quality {
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_LOW => 4,
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_MEDIUM => 8,
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_HIGH => 16,
                MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_ULTRA => 32,
                _ => unreachable!(), // Not possible, since the mask is 2 bits, and we've covered all 4 cases
            },
        ));

        if key.contains(MeshPipelineKey::VISIBILITY_RANGE_DITHER) {
            shader_defs.push("VISIBILITY_RANGE_DITHER".into());
        }

        if key.contains(MeshPipelineKey::DISTANCE_FOG) {
            shader_defs.push("DISTANCE_FOG".into());
        }

        if self.binding_arrays_are_usable {
            shader_defs.push("MULTIPLE_LIGHT_PROBES_IN_ARRAY".into());
            shader_defs.push("MULTIPLE_LIGHTMAPS_IN_ARRAY".into());
        }

        if IRRADIANCE_VOLUMES_ARE_USABLE {
            shader_defs.push("IRRADIANCE_VOLUMES_ARE_USABLE".into());
        }

        if self.clustered_decals_are_usable {
            shader_defs.push("CLUSTERED_DECALS_ARE_USABLE".into());
            if cfg!(feature = "pbr_light_textures") {
                shader_defs.push("LIGHT_TEXTURES".into());
            }
        }

        let format = if key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        // This is defined here so that custom shaders that use something other than
        // the mesh binding from bevy_pbr::mesh_bindings can easily make use of this
        // in their own shaders.
        if let Some(per_object_buffer_batch_size) = self.per_object_buffer_batch_size {
            shader_defs.push(ShaderDefVal::UInt(
                "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
                per_object_buffer_batch_size,
            ));
        }

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            layout: bind_group_layout,
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                topology: key.primitive_topology(),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
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
                alpha_to_coverage_enabled,
            },
            label: Some(label),
            ..default()
        })
    }
}

pub fn is_skinned(layout: &MeshVertexBufferLayoutRef) -> bool {
    layout.0.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
        && layout.0.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
}

pub fn setup_morph_and_skinning_defs(
    mesh_layouts: &MeshLayouts,
    layout: &MeshVertexBufferLayoutRef,
    offset: u32,
    key: &MeshPipelineKey,
    shader_defs: &mut Vec<ShaderDefVal>,
    vertex_attributes: &mut Vec<VertexAttributeDescriptor>,
    skins_use_uniform_buffers: bool,
) -> BindGroupLayoutDescriptor {
    let is_morphed = key.intersects(MeshPipelineKey::MORPH_TARGETS);
    let is_lightmapped = key.intersects(MeshPipelineKey::LIGHTMAPPED);
    let motion_vector_prepass = key.intersects(MeshPipelineKey::MOTION_VECTOR_PREPASS);

    if skins_use_uniform_buffers {
        shader_defs.push("SKINS_USE_UNIFORM_BUFFERS".into());
    }

    let mut add_skin_data = || {
        shader_defs.push("SKINNED".into());
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(offset));
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(offset + 1));
    };

    match (
        is_skinned(layout),
        is_morphed,
        is_lightmapped,
        motion_vector_prepass,
    ) {
        (true, false, _, true) => {
            add_skin_data();
            mesh_layouts.skinned_motion.clone()
        }
        (true, false, _, false) => {
            add_skin_data();
            mesh_layouts.skinned.clone()
        }
        (true, true, _, true) => {
            add_skin_data();
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_skinned_motion.clone()
        }
        (true, true, _, false) => {
            add_skin_data();
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_skinned.clone()
        }
        (false, true, _, true) => {
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_motion.clone()
        }
        (false, true, _, false) => {
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed.clone()
        }
        (false, false, true, _) => mesh_layouts.lightmapped.clone(),
        (false, false, false, _) => mesh_layouts.model_only.clone(),
    }
}
