use bevy_app::Plugin;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetId, AssetServer, Handle};
use bevy_camera::{visibility::ViewVisibility, Camera2d};
use bevy_render::RenderStartup;
use bevy_shader::{load_shader_library, Shader, ShaderDefVal, ShaderSettings};

use crate::{tonemapping_pipeline_key, Material2dBindGroupId};
use bevy_core_pipeline::{
    core_2d::{AlphaMask2d, Opaque2d, Transparent2d, CORE_2D_DEPTH_FORMAT},
    tonemapping::{
        get_lut_bind_group_layout_entries, get_lut_bindings, DebandDither, Tonemapping,
        TonemappingLuts,
    },
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Tick;
use bevy_ecs::system::SystemChangeTick;
use bevy_ecs::{
    prelude::*,
    query::ROQueryItem,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_image::{BevyDefault, Image, ImageSampler, TextureFormatPixelInfo};
use bevy_math::{Affine3, Vec4};
use bevy_mesh::{Mesh, Mesh2d, MeshTag, MeshVertexBufferLayoutRef};
use bevy_render::prelude::Msaa;
use bevy_render::RenderSystems::PrepareAssets;
use bevy_render::{
    batching::{
        gpu_preprocessing::IndirectParametersCpuMetadata,
        no_gpu_preprocessing::{
            self, batch_and_prepare_binned_render_phase, batch_and_prepare_sorted_render_phase,
            write_batched_instance_buffer, BatchedInstanceBuffer,
        },
        GetBatchData, GetFullBatchData, NoAutomaticBatching,
    },
    globals::{GlobalsBuffer, GlobalsUniform},
    mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
    render_asset::RenderAssets,
    render_phase::{
        sweep_old_entities, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
        TrackedRenderPass,
    },
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, MainEntityHashMap},
    texture::{DefaultImageSampler, FallbackImage, GpuImage},
    view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::default;
use nonmax::NonMaxU32;
use tracing::error;

#[derive(Default)]
pub struct Mesh2dRenderPlugin;

impl Plugin for Mesh2dRenderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_shader_library!(app, "mesh2d_vertex_output.wgsl");
        load_shader_library!(app, "mesh2d_view_types.wgsl");
        load_shader_library!(app, "mesh2d_view_bindings.wgsl");
        load_shader_library!(app, "mesh2d_types.wgsl");
        load_shader_library!(app, "mesh2d_functions.wgsl");

        embedded_asset!(app, "mesh2d.wgsl");

        // These bindings should be loaded as a shader library, but it depends on runtime
        // information, so we will load it in a system.
        embedded_asset!(app, "mesh2d_bindings.wgsl");

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ViewKeyCache>()
                .init_resource::<RenderMesh2dInstances>()
                .init_resource::<SpecializedMeshPipelines<Mesh2dPipeline>>()
                .init_resource::<ViewSpecializationTicks>()
                .add_systems(
                    RenderStartup,
                    (
                        init_mesh_2d_pipeline,
                        init_batched_instance_buffer,
                        load_mesh2d_bindings,
                    ),
                )
                .add_systems(ExtractSchedule, extract_mesh2d)
                .add_systems(
                    Render,
                    (
                        check_views_need_specialization.in_set(PrepareAssets),
                        (
                            sweep_old_entities::<Opaque2d>,
                            sweep_old_entities::<AlphaMask2d>,
                        )
                            .in_set(RenderSystems::QueueSweep),
                        batch_and_prepare_binned_render_phase::<Opaque2d, Mesh2dPipeline>
                            .in_set(RenderSystems::PrepareResources),
                        batch_and_prepare_binned_render_phase::<AlphaMask2d, Mesh2dPipeline>
                            .in_set(RenderSystems::PrepareResources),
                        batch_and_prepare_sorted_render_phase::<Transparent2d, Mesh2dPipeline>
                            .in_set(RenderSystems::PrepareResources),
                        write_batched_instance_buffer::<Mesh2dPipeline>
                            .in_set(RenderSystems::PrepareResourcesFlush),
                        prepare_mesh2d_bind_group.in_set(RenderSystems::PrepareBindGroups),
                        prepare_mesh2d_view_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                        no_gpu_preprocessing::clear_batched_cpu_instance_buffers::<Mesh2dPipeline>
                            .in_set(RenderSystems::Cleanup)
                            .after(RenderSystems::Render),
                    ),
                );
        }
    }
}

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewKeyCache(MainEntityHashMap<Mesh2dPipelineKey>);

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewSpecializationTicks(MainEntityHashMap<Tick>);

pub fn check_views_need_specialization(
    mut view_key_cache: ResMut<ViewKeyCache>,
    mut view_specialization_ticks: ResMut<ViewSpecializationTicks>,
    views: Query<(
        &MainEntity,
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
    ticks: SystemChangeTick,
) {
    for (view_entity, view, msaa, tonemapping, dither) in &views {
        let mut view_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= Mesh2dPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= Mesh2dPipelineKey::DEBAND_DITHER;
            }
        }

        if !view_key_cache
            .get_mut(view_entity)
            .is_some_and(|current_key| *current_key == view_key)
        {
            view_key_cache.insert(*view_entity, view_key);
            view_specialization_ticks.insert(*view_entity, ticks.this_run());
        }
    }
}

pub fn init_batched_instance_buffer(mut commands: Commands, render_device: Res<RenderDevice>) {
    commands.insert_resource(BatchedInstanceBuffer::<Mesh2dUniform>::new(&render_device));
}

fn load_mesh2d_bindings(render_device: Res<RenderDevice>, asset_server: Res<AssetServer>) {
    let mut mesh_bindings_shader_defs = Vec::with_capacity(1);

    if let Some(per_object_buffer_batch_size) =
        GpuArrayBuffer::<Mesh2dUniform>::batch_size(&render_device)
    {
        mesh_bindings_shader_defs.push(ShaderDefVal::UInt(
            "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
            per_object_buffer_batch_size,
        ));
    }

    // Load the mesh_bindings shader module here as it depends on runtime information about
    // whether storage buffers are supported, or the maximum uniform buffer binding size.
    let handle: Handle<Shader> = load_embedded_asset!(
        asset_server.as_ref(),
        "mesh2d_bindings.wgsl",
        move |settings| {
            *settings = ShaderSettings {
                shader_defs: mesh_bindings_shader_defs.clone(),
            }
        }
    );
    // Forget the handle so we don't have to store it anywhere, and we keep the embedded asset
    // loaded. Note: This is what happens in `load_shader_library` internally.
    core::mem::forget(handle);
}

#[derive(Component)]
pub struct Mesh2dTransforms {
    pub world_from_local: Affine3,
    pub flags: u32,
}

#[derive(ShaderType, Clone, Copy)]
pub struct Mesh2dUniform {
    // Affine 4x3 matrix transposed to 3x4
    pub world_from_local: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub local_from_world_transpose_a: [Vec4; 2],
    pub local_from_world_transpose_b: f32,
    pub flags: u32,
    pub tag: u32,
}

impl Mesh2dUniform {
    fn from_components(mesh_transforms: &Mesh2dTransforms, tag: u32) -> Self {
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            mesh_transforms.world_from_local.inverse_transpose_3x3();
        Self {
            world_from_local: mesh_transforms.world_from_local.to_transpose(),
            local_from_world_transpose_a,
            local_from_world_transpose_b,
            flags: mesh_transforms.flags,
            tag,
        }
    }
}

// NOTE: These must match the bit flags in bevy_sprite_render/src/mesh2d/mesh2d.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

pub struct RenderMesh2dInstance {
    pub transforms: Mesh2dTransforms,
    pub mesh_asset_id: AssetId<Mesh>,
    pub material_bind_group_id: Material2dBindGroupId,
    pub automatic_batching: bool,
    pub tag: u32,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderMesh2dInstances(MainEntityHashMap<RenderMesh2dInstance>);

#[derive(Component, Default)]
pub struct Mesh2dMarker;

pub fn extract_mesh2d(
    mut render_mesh_instances: ResMut<RenderMesh2dInstances>,
    query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            &Mesh2d,
            Option<&MeshTag>,
            Has<NoAutomaticBatching>,
        )>,
    >,
) {
    render_mesh_instances.clear();

    for (entity, view_visibility, transform, handle, tag, no_automatic_batching) in &query {
        if !view_visibility.get() {
            continue;
        }
        render_mesh_instances.insert(
            entity.into(),
            RenderMesh2dInstance {
                transforms: Mesh2dTransforms {
                    world_from_local: (&transform.affine()).into(),
                    flags: MeshFlags::empty().bits(),
                },
                mesh_asset_id: handle.0.id(),
                material_bind_group_id: Material2dBindGroupId::default(),
                automatic_batching: !no_automatic_batching,
                tag: tag.map_or(0, |i| **i),
            },
        );
    }
}

#[derive(Resource, Clone)]
pub struct Mesh2dPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub shader: Handle<Shader>,
    // This dummy white texture is to be used in place of optional textures
    pub dummy_white_gpu_image: GpuImage,
    pub per_object_buffer_batch_size: Option<u32>,
}

pub fn init_mesh_2d_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    default_sampler: Res<DefaultImageSampler>,
    asset_server: Res<AssetServer>,
) {
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
    let view_layout = render_device.create_bind_group_layout(
        "mesh2d_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<GlobalsUniform>(false),
                tonemapping_lut_entries[0].visibility(ShaderStages::FRAGMENT),
                tonemapping_lut_entries[1].visibility(ShaderStages::FRAGMENT),
            ),
        ),
    );

    let mesh_layout = render_device.create_bind_group_layout(
        "mesh2d_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            GpuArrayBuffer::<Mesh2dUniform>::binding_layout(&render_device),
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
    commands.insert_resource(Mesh2dPipeline {
        view_layout,
        mesh_layout,
        dummy_white_gpu_image,
        per_object_buffer_batch_size: GpuArrayBuffer::<Mesh2dUniform>::batch_size(&render_device),
        shader: load_embedded_asset!(asset_server.as_ref(), "mesh2d.wgsl"),
    });
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
    pub struct Mesh2dPipelineKey: u32 {
        const NONE                              = 0;
        const HDR                               = 1 << 0;
        const TONEMAP_IN_SHADER                 = 1 << 1;
        const DEBAND_DITHER                     = 1 << 2;
        const BLEND_ALPHA                       = 1 << 3;
        const MAY_DISCARD                       = 1 << 4;
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
        const PRIMITIVE_TOPOLOGY_RESERVED_BITS  = Self::PRIMITIVE_TOPOLOGY_MASK_BITS << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        const TONEMAP_METHOD_RESERVED_BITS      = Self::TONEMAP_METHOD_MASK_BITS << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_NONE               = 0 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD           = 1 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_REINHARD_LUMINANCE = 2 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_ACES_FITTED        = 3 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_AGX                = 4 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM = 5 << Self::TONEMAP_METHOD_SHIFT_BITS;
        const TONEMAP_METHOD_TONY_MC_MAPFACE    = 6 << Self::TONEMAP_METHOD_SHIFT_BITS;
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

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    pub fn from_hdr(hdr: bool) -> Self {
        if hdr {
            Mesh2dPipelineKey::HDR
        } else {
            Mesh2dPipelineKey::NONE
        }
    }

    pub fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u32)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits)
    }

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
            layout: vec![self.view_layout.clone(), self.mesh_layout.clone()],
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
            ..default()
        })
    }
}

#[derive(Resource)]
pub struct Mesh2dBindGroup {
    pub value: BindGroup,
}

pub fn prepare_mesh2d_bind_group(
    mut commands: Commands,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    render_device: Res<RenderDevice>,
    mesh2d_uniforms: Res<BatchedInstanceBuffer<Mesh2dUniform>>,
) {
    if let Some(binding) = mesh2d_uniforms.instance_data_binding() {
        commands.insert_resource(Mesh2dBindGroup {
            value: render_device.create_bind_group(
                "mesh2d_bind_group",
                &mesh2d_pipeline.mesh_layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

#[derive(Component)]
pub struct Mesh2dViewBindGroup {
    pub value: BindGroup,
}

pub fn prepare_mesh2d_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &Tonemapping), (With<ExtractedView>, With<Camera2d>)>,
    globals_buffer: Res<GlobalsBuffer>,
    tonemapping_luts: Res<TonemappingLuts>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
) {
    let (Some(view_binding), Some(globals)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) else {
        return;
    };

    for (entity, tonemapping) in &views {
        let lut_bindings =
            get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
        let view_bind_group = render_device.create_bind_group(
            "mesh2d_view_bind_group",
            &mesh2d_pipeline.view_layout,
            &BindGroupEntries::sequential((
                view_binding.clone(),
                globals.clone(),
                lut_bindings.0,
                lut_bindings.1,
            )),
        );

        commands.entity(entity).insert(Mesh2dViewBindGroup {
            value: view_bind_group,
        });
    }
}

pub struct SetMesh2dViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMesh2dViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<ViewUniformOffset>, Read<Mesh2dViewBindGroup>);
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, mesh2d_view_bind_group): ROQueryItem<'w, '_, Self::ViewQuery>,
        _view: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &mesh2d_view_bind_group.value, &[view_uniform.offset]);

        RenderCommandResult::Success
    }
}

pub struct SetMesh2dBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMesh2dBindGroup<I> {
    type Param = SRes<Mesh2dBindGroup>;
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        mesh2d_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mut dynamic_offsets: [u32; 1] = Default::default();
        let mut offset_count = 0;
        if let PhaseItemExtraIndex::DynamicOffset(dynamic_offset) = item.extra_index() {
            dynamic_offsets[offset_count] = dynamic_offset;
            offset_count += 1;
        }
        pass.set_bind_group(
            I,
            &mesh2d_bind_group.into_inner().value,
            &dynamic_offsets[..offset_count],
        );
        RenderCommandResult::Success
    }
}

pub struct DrawMesh2d;
impl<P: PhaseItem> RenderCommand<P> for DrawMesh2d {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMesh2dInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (meshes, render_mesh2d_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
            render_mesh2d_instances.get(&item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.get(*mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

        let batch_range = item.batch_range();
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);

                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    batch_range.clone(),
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, batch_range.clone());
            }
        }
        RenderCommandResult::Success
    }
}
