use crate::{
    environment_map, prepass, EnvironmentMapLight, FogMeta, GlobalLightMeta, GpuFog, GpuLights,
    GpuPointLights, LightMeta, MaterialBindGroupId, NotShadowCaster, NotShadowReceiver,
    PreviousGlobalTransform, ScreenSpaceAmbientOcclusionTextures, Shadow, ShadowSamplers,
    ViewClusterBindings, ViewFogUniformOffset, ViewLightsUniformOffset, ViewShadowBindings,
    CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT, MAX_CASCADES_PER_LIGHT, MAX_DIRECTIONAL_LIGHTS,
};
use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_core_pipeline::{
    core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    prepass::ViewPrepassTextures,
    tonemapping::{
        get_lut_bind_group_layout_entries, get_lut_bindings, Tonemapping, TonemappingLuts,
    },
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::*,
    query::{QueryItem, ROQueryItem},
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_math::{Affine3, Vec2, Vec4};
use bevy_render::{
    batching::{
        batch_and_prepare_render_phase, write_batched_instance_buffer, GetBatchData,
        NoAutomaticBatching,
    },
    globals::{GlobalsBuffer, GlobalsUniform},
    mesh::{
        GpuBufferInfo, InnerMeshVertexBufferLayout, Mesh, MeshVertexBufferLayout,
        VertexAttributeDescriptor,
    },
    prelude::Msaa,
    render_asset::RenderAssets,
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{
        BevyDefault, DefaultImageSampler, FallbackImageCubemap, FallbackImagesDepth,
        FallbackImagesMsaa, GpuImage, Image, ImageSampler, TextureFormatPixelInfo,
    },
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms, ViewVisibility},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{tracing::error, EntityHashMap, HashMap, Hashed};
use std::cell::Cell;
use thread_local::ThreadLocal;

use crate::render::{
    morph::{
        extract_morphs, no_automatic_morph_batching, prepare_morphs, MorphIndices, MorphUniform,
    },
    skin::{extract_skins, no_automatic_skin_batching, prepare_skins, SkinUniform},
    MeshLayouts,
};

use super::skin::SkinIndices;

#[derive(Default)]
pub struct MeshRenderPlugin;

pub const MESH_VERTEX_OUTPUT: Handle<Shader> = Handle::weak_from_u128(2645551199423808407);
pub const MESH_VIEW_TYPES_HANDLE: Handle<Shader> = Handle::weak_from_u128(8140454348013264787);
pub const MESH_VIEW_BINDINGS_HANDLE: Handle<Shader> = Handle::weak_from_u128(9076678235888822571);
pub const MESH_TYPES_HANDLE: Handle<Shader> = Handle::weak_from_u128(2506024101911992377);
pub const MESH_BINDINGS_HANDLE: Handle<Shader> = Handle::weak_from_u128(16831548636314682308);
pub const MESH_FUNCTIONS_HANDLE: Handle<Shader> = Handle::weak_from_u128(6300874327833745635);
pub const MESH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(3252377289100772450);
pub const SKINNING_HANDLE: Handle<Shader> = Handle::weak_from_u128(13215291596265391738);
pub const MORPH_HANDLE: Handle<Shader> = Handle::weak_from_u128(970982813587607345);

impl Plugin for MeshRenderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            MESH_VERTEX_OUTPUT,
            "mesh_vertex_output.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH_VIEW_TYPES_HANDLE,
            "mesh_view_types.wgsl",
            Shader::from_wgsl_with_defs,
            vec![
                ShaderDefVal::UInt(
                    "MAX_DIRECTIONAL_LIGHTS".into(),
                    MAX_DIRECTIONAL_LIGHTS as u32
                ),
                ShaderDefVal::UInt(
                    "MAX_CASCADES_PER_LIGHT".into(),
                    MAX_CASCADES_PER_LIGHT as u32,
                )
            ]
        );
        load_internal_asset!(
            app,
            MESH_VIEW_BINDINGS_HANDLE,
            "mesh_view_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, MESH_TYPES_HANDLE, "mesh_types.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            MESH_FUNCTIONS_HANDLE,
            "mesh_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, MESH_SHADER_HANDLE, "mesh.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, SKINNING_HANDLE, "skinning.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, MORPH_HANDLE, "morph.wgsl", Shader::from_wgsl);

        app.add_systems(
            PostUpdate,
            (no_automatic_skin_batching, no_automatic_morph_batching),
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderMeshInstances>()
                .init_resource::<MeshBindGroups>()
                .init_resource::<SkinUniform>()
                .init_resource::<SkinIndices>()
                .init_resource::<MorphUniform>()
                .init_resource::<MorphIndices>()
                .add_systems(
                    ExtractSchedule,
                    (extract_meshes, extract_skins, extract_morphs),
                )
                .add_systems(
                    Render,
                    (
                        (
                            batch_and_prepare_render_phase::<Opaque3d, MeshPipeline>,
                            batch_and_prepare_render_phase::<Transparent3d, MeshPipeline>,
                            batch_and_prepare_render_phase::<AlphaMask3d, MeshPipeline>,
                            batch_and_prepare_render_phase::<Shadow, MeshPipeline>,
                        )
                            .in_set(RenderSet::PrepareResources),
                        write_batched_instance_buffer::<MeshPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                        prepare_skins.in_set(RenderSet::PrepareResources),
                        prepare_morphs.in_set(RenderSet::PrepareResources),
                        prepare_mesh_bind_group.in_set(RenderSet::PrepareBindGroups),
                        prepare_mesh_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let mut mesh_bindings_shader_defs = Vec::with_capacity(1);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            if let Some(per_object_buffer_batch_size) = GpuArrayBuffer::<MeshUniform>::batch_size(
                render_app.world.resource::<RenderDevice>(),
            ) {
                mesh_bindings_shader_defs.push(ShaderDefVal::UInt(
                    "PER_OBJECT_BUFFER_BATCH_SIZE".into(),
                    per_object_buffer_batch_size,
                ));
            }

            render_app
                .insert_resource(GpuArrayBuffer::<MeshUniform>::new(
                    render_app.world.resource::<RenderDevice>(),
                ))
                .init_resource::<MeshPipeline>();
        }

        // Load the mesh_bindings shader module here as it depends on runtime information about
        // whether storage buffers are supported, or the maximum uniform buffer binding size.
        load_internal_asset!(
            app,
            MESH_BINDINGS_HANDLE,
            "mesh_bindings.wgsl",
            Shader::from_wgsl_with_defs,
            mesh_bindings_shader_defs
        );
    }
}

#[derive(Component)]
pub struct MeshTransforms {
    pub transform: Affine3,
    pub previous_transform: Affine3,
    pub flags: u32,
}

#[derive(ShaderType, Clone)]
pub struct MeshUniform {
    // Affine 4x3 matrices transposed to 3x4
    pub transform: [Vec4; 3],
    pub previous_transform: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub inverse_transpose_model_a: [Vec4; 2],
    pub inverse_transpose_model_b: f32,
    pub flags: u32,
}

impl From<&MeshTransforms> for MeshUniform {
    fn from(mesh_transforms: &MeshTransforms) -> Self {
        let (inverse_transpose_model_a, inverse_transpose_model_b) =
            mesh_transforms.transform.inverse_transpose_3x3();
        Self {
            transform: mesh_transforms.transform.to_transpose(),
            previous_transform: mesh_transforms.previous_transform.to_transpose(),
            inverse_transpose_model_a,
            inverse_transpose_model_b,
            flags: mesh_transforms.flags,
        }
    }
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_types.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        const SHADOW_RECEIVER            = (1 << 0);
        // Indicates the sign of the determinant of the 3x3 model matrix. If the sign is positive,
        // then the flag should be set, else it should not be set.
        const SIGN_DETERMINANT_MODEL_3X3 = (1 << 31);
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

pub struct RenderMeshInstance {
    pub transforms: MeshTransforms,
    pub mesh_asset_id: AssetId<Mesh>,
    pub material_bind_group_id: MaterialBindGroupId,
    pub shadow_caster: bool,
    pub automatic_batching: bool,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderMeshInstances(EntityHashMap<Entity, RenderMeshInstance>);

#[derive(Component)]
pub struct Mesh3d;

pub fn extract_meshes(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    mut thread_local_queues: Local<ThreadLocal<Cell<Vec<(Entity, RenderMeshInstance)>>>>,
    meshes_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
            &Handle<Mesh>,
            Has<NotShadowReceiver>,
            Has<NotShadowCaster>,
            Has<NoAutomaticBatching>,
        )>,
    >,
) {
    meshes_query.par_iter().for_each(
        |(
            entity,
            view_visibility,
            transform,
            previous_transform,
            handle,
            not_receiver,
            not_caster,
            no_automatic_batching,
        )| {
            if !view_visibility.get() {
                return;
            }
            let transform = transform.affine();
            let previous_transform = previous_transform.map(|t| t.0).unwrap_or(transform);
            let mut flags = if not_receiver {
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
            let tls = thread_local_queues.get_or_default();
            let mut queue = tls.take();
            queue.push((
                entity,
                RenderMeshInstance {
                    mesh_asset_id: handle.id(),
                    transforms,
                    shadow_caster: !not_caster,
                    material_bind_group_id: MaterialBindGroupId::default(),
                    automatic_batching: !no_automatic_batching,
                },
            ));
            tls.set(queue);
        },
    );

    render_mesh_instances.clear();
    let mut entities = Vec::with_capacity(*previous_len);
    for queue in thread_local_queues.iter_mut() {
        // FIXME: Remove this - it is just a workaround to enable rendering to work as
        // render commands require an entity to exist at the moment.
        entities.extend(queue.get_mut().iter().map(|(e, _)| (*e, Mesh3d)));
        render_mesh_instances.extend(queue.get_mut().drain(..));
    }
    *previous_len = entities.len();
    commands.insert_or_spawn_batch(entities);
}

#[derive(Resource, Clone)]
pub struct MeshPipeline {
    pub view_layout: BindGroupLayout,
    pub view_layout_multisampled: BindGroupLayout,
    // This dummy white texture is to be used in place of optional StandardMaterial textures
    pub dummy_white_gpu_image: GpuImage,
    pub clustered_forward_buffer_binding_type: BufferBindingType,
    pub mesh_layouts: MeshLayouts,
    /// `MeshUniform`s are stored in arrays in buffers. If storage buffers are available, they
    /// are used and this will be `None`, otherwise uniform buffers will be used with batches
    /// of this many `MeshUniform`s, stored at dynamic offsets within the uniform buffer.
    /// Use code like this in custom shaders:
    /// ```wgsl
    /// ##ifdef PER_OBJECT_BUFFER_BATCH_SIZE
    /// @group(2) @binding(0) var<uniform> mesh: array<Mesh, #{PER_OBJECT_BUFFER_BATCH_SIZE}u>;
    /// ##else
    /// @group(2) @binding(0) var<storage> mesh: array<Mesh>;
    /// ##endif // PER_OBJECT_BUFFER_BATCH_SIZE
    /// ```
    pub per_object_buffer_batch_size: Option<u32>,
}

impl FromWorld for MeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);
        let clustered_forward_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);

        /// Returns the appropriate bind group layout vec based on the parameters
        fn layout_entries(
            clustered_forward_buffer_binding_type: BufferBindingType,
            multisampled: bool,
        ) -> Vec<BindGroupLayoutEntry> {
            let mut entries = vec![
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(ViewUniform::min_size()),
                    },
                    count: None,
                },
                // Lights
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(GpuLights::min_size()),
                    },
                    count: None,
                },
                // Point Shadow Texture Cube Array
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Depth,
                        #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                        view_dimension: TextureViewDimension::CubeArray,
                        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                        view_dimension: TextureViewDimension::Cube,
                    },
                    count: None,
                },
                // Point Shadow Texture Array Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                },
                // Directional Shadow Texture Array
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Depth,
                        #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                        view_dimension: TextureViewDimension::D2Array,
                        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Directional Shadow Texture Array Sampler
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                },
                // PointLights
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: clustered_forward_buffer_binding_type,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuPointLights::min_size(
                            clustered_forward_buffer_binding_type,
                        )),
                    },
                    count: None,
                },
                // ClusteredLightIndexLists
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: clustered_forward_buffer_binding_type,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            ViewClusterBindings::min_size_cluster_light_index_lists(
                                clustered_forward_buffer_binding_type,
                            ),
                        ),
                    },
                    count: None,
                },
                // ClusterOffsetsAndCounts
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: clustered_forward_buffer_binding_type,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            ViewClusterBindings::min_size_cluster_offsets_and_counts(
                                clustered_forward_buffer_binding_type,
                            ),
                        ),
                    },
                    count: None,
                },
                // Globals
                BindGroupLayoutEntry {
                    binding: 9,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GlobalsUniform::min_size()),
                    },
                    count: None,
                },
                // Fog
                BindGroupLayoutEntry {
                    binding: 10,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(GpuFog::min_size()),
                    },
                    count: None,
                },
                // Screen space ambient occlusion texture
                BindGroupLayoutEntry {
                    binding: 11,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
            ];

            // EnvironmentMapLight
            let environment_map_entries =
                environment_map::get_bind_group_layout_entries([12, 13, 14]);
            entries.extend_from_slice(&environment_map_entries);

            // Tonemapping
            let tonemapping_lut_entries = get_lut_bind_group_layout_entries([15, 16]);
            entries.extend_from_slice(&tonemapping_lut_entries);

            if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32")))
                || (cfg!(all(feature = "webgl", target_arch = "wasm32")) && !multisampled)
            {
                entries.extend_from_slice(&prepass::get_bind_group_layout_entries(
                    [17, 18, 19],
                    multisampled,
                ));
            }

            entries
        }

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("mesh_view_layout"),
            entries: &layout_entries(clustered_forward_buffer_binding_type, false),
        });

        let view_layout_multisampled =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("mesh_view_layout_multisampled"),
                entries: &layout_entries(clustered_forward_buffer_binding_type, true),
            });

        // A 1x1x1 'all 1.0' texture to use as a dummy texture to use in place of optional StandardMaterial textures
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler_descriptor {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(descriptor) => render_device.create_sampler(&descriptor),
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.texture_descriptor.size.width * format_size as u32),
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
                size: Vec2::new(
                    image.texture_descriptor.size.width as f32,
                    image.texture_descriptor.size.height as f32,
                ),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        MeshPipeline {
            view_layout,
            view_layout_multisampled,
            clustered_forward_buffer_binding_type,
            dummy_white_gpu_image,
            mesh_layouts: MeshLayouts::new(&render_device),
            per_object_buffer_batch_size: GpuArrayBuffer::<MeshUniform>::batch_size(&render_device),
        }
    }
}

impl MeshPipeline {
    pub fn get_image_texture<'a>(
        &'a self,
        gpu_images: &'a RenderAssets<Image>,
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

impl GetBatchData for MeshPipeline {
    type Param = SRes<RenderMeshInstances>;
    type Query = Entity;
    type QueryFilter = With<Mesh3d>;
    type CompareData = (MaterialBindGroupId, AssetId<Mesh>);
    type BufferData = MeshUniform;

    fn get_batch_data(
        mesh_instances: &SystemParamItem<Self::Param>,
        entity: &QueryItem<Self::Query>,
    ) -> (Self::BufferData, Option<Self::CompareData>) {
        let mesh_instance = mesh_instances
            .get(entity)
            .expect("Failed to find render mesh instance");
        (
            (&mesh_instance.transforms).into(),
            mesh_instance.automatic_batching.then_some((
                mesh_instance.material_bind_group_id,
                mesh_instance.mesh_asset_id,
            )),
        )
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    /// MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct MeshPipelineKey: u32 {
        const NONE                              = 0;
        const HDR                               = (1 << 0);
        const TONEMAP_IN_SHADER                 = (1 << 1);
        const DEBAND_DITHER                     = (1 << 2);
        const DEPTH_PREPASS                     = (1 << 3);
        const NORMAL_PREPASS                    = (1 << 4);
        const MOTION_VECTOR_PREPASS             = (1 << 5);
        const MAY_DISCARD                       = (1 << 6); // Guards shader codepaths that may discard, allowing early depth tests in most cases
                                                            // See: https://www.khronos.org/opengl/wiki/Early_Fragment_Test
        const ENVIRONMENT_MAP                   = (1 << 7);
        const SCREEN_SPACE_AMBIENT_OCCLUSION    = (1 << 8);
        const DEPTH_CLAMP_ORTHO                 = (1 << 9);
        const TAA                               = (1 << 10);
        const MORPH_TARGETS                     = (1 << 11);
        const BLEND_RESERVED_BITS               = Self::BLEND_MASK_BITS << Self::BLEND_SHIFT_BITS; // ← Bitmask reserving bits for the blend state
        const BLEND_OPAQUE                      = (0 << Self::BLEND_SHIFT_BITS);                   // ← Values are just sequential within the mask, and can range from 0 to 3
        const BLEND_PREMULTIPLIED_ALPHA         = (1 << Self::BLEND_SHIFT_BITS);                   //
        const BLEND_MULTIPLY                    = (2 << Self::BLEND_SHIFT_BITS);                   // ← We still have room for one more value without adding more bits
        const BLEND_ALPHA                       = (3 << Self::BLEND_SHIFT_BITS);
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

impl MeshPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();
    const PRIMITIVE_TOPOLOGY_MASK_BITS: u32 = 0b111;
    const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u32 =
        Self::MSAA_SHIFT_BITS - Self::PRIMITIVE_TOPOLOGY_MASK_BITS.count_ones();
    const BLEND_MASK_BITS: u32 = 0b11;
    const BLEND_SHIFT_BITS: u32 =
        Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS - Self::BLEND_MASK_BITS.count_ones();
    const TONEMAP_METHOD_MASK_BITS: u32 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u32 =
        Self::BLEND_SHIFT_BITS - Self::TONEMAP_METHOD_MASK_BITS.count_ones();

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    pub fn from_hdr(hdr: bool) -> Self {
        if hdr {
            MeshPipelineKey::HDR
        } else {
            MeshPipelineKey::NONE
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

fn is_skinned(layout: &Hashed<InnerMeshVertexBufferLayout>) -> bool {
    layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX) && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
}
pub fn setup_morph_and_skinning_defs(
    mesh_layouts: &MeshLayouts,
    layout: &Hashed<InnerMeshVertexBufferLayout>,
    offset: u32,
    key: &MeshPipelineKey,
    shader_defs: &mut Vec<ShaderDefVal>,
    vertex_attributes: &mut Vec<VertexAttributeDescriptor>,
) -> BindGroupLayout {
    let mut add_skin_data = || {
        shader_defs.push("SKINNED".into());
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(offset));
        vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(offset + 1));
    };
    let is_morphed = key.intersects(MeshPipelineKey::MORPH_TARGETS);
    match (is_skinned(layout), is_morphed) {
        (true, false) => {
            add_skin_data();
            mesh_layouts.skinned.clone()
        }
        (true, true) => {
            add_skin_data();
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed_skinned.clone()
        }
        (false, true) => {
            shader_defs.push("MORPH_TARGETS".into());
            mesh_layouts.morphed.clone()
        }
        (false, false) => mesh_layouts.model_only.clone(),
    }
}

impl SpecializedMeshPipeline for MeshPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        shader_defs.push("VERTEX_OUTPUT_INSTANCE_INDEX".into());

        if layout.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        if layout.contains(Mesh::ATTRIBUTE_NORMAL) {
            shader_defs.push("VERTEX_NORMALS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(1));
        }

        if layout.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.contains(Mesh::ATTRIBUTE_UV_1) {
            shader_defs.push("VERTEX_UVS_1".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_1.at_shader_location(3));
        }

        if layout.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push("VERTEX_TANGENTS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(4));
        }

        if layout.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(5));
        }

        let mut bind_group_layout = match key.msaa_samples() {
            1 => vec![self.view_layout.clone()],
            _ => {
                shader_defs.push("MULTISAMPLED".into());
                vec![self.view_layout_multisampled.clone()]
            }
        };

        bind_group_layout.push(setup_morph_and_skinning_defs(
            &self.mesh_layouts,
            layout,
            6,
            &key,
            &mut shader_defs,
            &mut vertex_attributes,
        ));

        if key.contains(MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION) {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        let (label, blend, depth_write_enabled);
        let pass = key.intersection(MeshPipelineKey::BLEND_RESERVED_BITS);
        let mut is_opaque = false;
        if pass == MeshPipelineKey::BLEND_ALPHA {
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
        } else {
            label = "opaque_mesh_pipeline".into();
            blend = Some(BlendState::REPLACE);
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
            is_opaque = true;
        }

        if key.contains(MeshPipelineKey::NORMAL_PREPASS) && key.msaa_samples() == 1 && is_opaque {
            shader_defs.push("LOAD_PREPASS_NORMALS".into());
        }

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED ".into());
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

        if key.contains(MeshPipelineKey::TAA) {
            shader_defs.push("TAA".into());
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

        let mut push_constant_ranges = Vec::with_capacity(1);
        if cfg!(all(feature = "webgl", target_arch = "wasm32")) {
            push_constant_ranges.push(PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..4,
            });
        }

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: MESH_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: MESH_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: bind_group_layout,
            push_constant_ranges,
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
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
            label: Some(label),
        })
    }
}

/// Bind groups for meshes currently loaded.
#[derive(Resource, Default)]
pub struct MeshBindGroups {
    model_only: Option<BindGroup>,
    skinned: Option<BindGroup>,
    morph_targets: HashMap<AssetId<Mesh>, BindGroup>,
}
impl MeshBindGroups {
    pub fn reset(&mut self) {
        self.model_only = None;
        self.skinned = None;
        self.morph_targets.clear();
    }
    /// Get the `BindGroup` for `GpuMesh` with given `handle_id`.
    pub fn get(
        &self,
        asset_id: AssetId<Mesh>,
        is_skinned: bool,
        morph: bool,
    ) -> Option<&BindGroup> {
        match (is_skinned, morph) {
            (_, true) => self.morph_targets.get(&asset_id),
            (true, false) => self.skinned.as_ref(),
            (false, false) => self.model_only.as_ref(),
        }
    }
}

pub fn prepare_mesh_bind_group(
    meshes: Res<RenderAssets<Mesh>>,
    mut groups: ResMut<MeshBindGroups>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
    mesh_uniforms: Res<GpuArrayBuffer<MeshUniform>>,
    skins_uniform: Res<SkinUniform>,
    weights_uniform: Res<MorphUniform>,
) {
    groups.reset();
    let layouts = &mesh_pipeline.mesh_layouts;
    let Some(model) = mesh_uniforms.binding() else {
        return;
    };
    groups.model_only = Some(layouts.model_only(&render_device, &model));

    let skin = skins_uniform.buffer.buffer();
    if let Some(skin) = skin {
        groups.skinned = Some(layouts.skinned(&render_device, &model, skin));
    }

    if let Some(weights) = weights_uniform.buffer.buffer() {
        for (id, gpu_mesh) in meshes.iter() {
            if let Some(targets) = gpu_mesh.morph_targets.as_ref() {
                let group = if let Some(skin) = skin.filter(|_| is_skinned(&gpu_mesh.layout)) {
                    layouts.morphed_skinned(&render_device, &model, skin, weights, targets)
                } else {
                    layouts.morphed(&render_device, &model, weights, targets)
                };
                groups.morph_targets.insert(id, group);
            }
        }
    }
}

#[derive(Component)]
pub struct MeshViewBindGroup {
    pub value: BindGroup,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_mesh_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh_pipeline: Res<MeshPipeline>,
    shadow_samplers: Res<ShadowSamplers>,
    light_meta: Res<LightMeta>,
    global_light_meta: Res<GlobalLightMeta>,
    fog_meta: Res<FogMeta>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(
        Entity,
        &ViewShadowBindings,
        &ViewClusterBindings,
        Option<&ScreenSpaceAmbientOcclusionTextures>,
        Option<&ViewPrepassTextures>,
        Option<&EnvironmentMapLight>,
        &Tonemapping,
    )>,
    images: Res<RenderAssets<Image>>,
    mut fallback_images: FallbackImagesMsaa,
    mut fallback_depths: FallbackImagesDepth,
    fallback_cubemap: Res<FallbackImageCubemap>,
    msaa: Res<Msaa>,
    globals_buffer: Res<GlobalsBuffer>,
    tonemapping_luts: Res<TonemappingLuts>,
) {
    if let (
        Some(view_binding),
        Some(light_binding),
        Some(point_light_binding),
        Some(globals),
        Some(fog_binding),
    ) = (
        view_uniforms.uniforms.binding(),
        light_meta.view_gpu_lights.binding(),
        global_light_meta.gpu_point_lights.binding(),
        globals_buffer.buffer.binding(),
        fog_meta.gpu_fogs.binding(),
    ) {
        for (
            entity,
            view_shadow_bindings,
            view_cluster_bindings,
            ssao_textures,
            prepass_textures,
            environment_map,
            tonemapping,
        ) in &views
        {
            let fallback_ssao = fallback_images
                .image_for_samplecount(1)
                .texture_view
                .clone();

            let layout = if msaa.samples() > 1 {
                &mesh_pipeline.view_layout_multisampled
            } else {
                &mesh_pipeline.view_layout
            };

            let mut entries = vec![
                BindGroupEntry {
                    binding: 0,
                    resource: view_binding.clone(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: light_binding.clone(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &view_shadow_bindings.point_light_depth_texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&shadow_samplers.point_light_sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &view_shadow_bindings.directional_light_depth_texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&shadow_samplers.directional_light_sampler),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: point_light_binding.clone(),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: view_cluster_bindings.light_index_lists_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: view_cluster_bindings.offsets_and_counts_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: globals.clone(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: fog_binding.clone(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(
                        ssao_textures
                            .map(|t| &t.screen_space_ambient_occlusion_texture.default_view)
                            .unwrap_or(&fallback_ssao),
                    ),
                },
            ];

            let env_map = environment_map::get_bindings(
                environment_map,
                &images,
                &fallback_cubemap,
                [12, 13, 14],
            );
            entries.extend_from_slice(&env_map);

            let tonemapping_luts =
                get_lut_bindings(&images, &tonemapping_luts, tonemapping, [15, 16]);
            entries.extend_from_slice(&tonemapping_luts);

            // When using WebGL, we can't have a depth texture with multisampling
            if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32")))
                || (cfg!(all(feature = "webgl", target_arch = "wasm32")) && msaa.samples() == 1)
            {
                entries.extend_from_slice(&prepass::get_bindings(
                    prepass_textures,
                    &mut fallback_images,
                    &mut fallback_depths,
                    &msaa,
                    [17, 18, 19],
                ));
            }

            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &entries,
                label: Some("mesh_view_bind_group"),
                layout,
            });

            commands.entity(entity).insert(MeshViewBindGroup {
                value: view_bind_group,
            });
        }
    }
}

pub struct SetMeshViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMeshViewBindGroup<I> {
    type Param = ();
    type ViewWorldQuery = (
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<ViewFogUniformOffset>,
        Read<MeshViewBindGroup>,
    );
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, view_lights, view_fog, mesh_view_bind_group): ROQueryItem<
            'w,
            Self::ViewWorldQuery,
        >,
        _entity: (),
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            &mesh_view_bind_group.value,
            &[view_uniform.offset, view_lights.offset, view_fog.offset],
        );

        RenderCommandResult::Success
    }
}

pub struct SetMeshBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMeshBindGroup<I> {
    type Param = (
        SRes<MeshBindGroups>,
        SRes<RenderMeshInstances>,
        SRes<SkinIndices>,
        SRes<MorphIndices>,
    );
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: (),
        (bind_groups, mesh_instances, skin_indices, morph_indices): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = bind_groups.into_inner();
        let mesh_instances = mesh_instances.into_inner();
        let skin_indices = skin_indices.into_inner();
        let morph_indices = morph_indices.into_inner();

        let entity = &item.entity();

        let Some(mesh) = mesh_instances.get(entity) else {
            return RenderCommandResult::Success;
        };
        let skin_index = skin_indices.get(entity);
        let morph_index = morph_indices.get(entity);

        let is_skinned = skin_index.is_some();
        let is_morphed = morph_index.is_some();

        let Some(bind_group) = bind_groups.get(mesh.mesh_asset_id, is_skinned, is_morphed) else {
            error!(
                "The MeshBindGroups resource wasn't set in the render phase. \
                It should be set by the queue_mesh_bind_group system.\n\
                This is a bevy bug! Please open an issue."
            );
            return RenderCommandResult::Failure;
        };

        let mut dynamic_offsets: [u32; 3] = Default::default();
        let mut offset_count = 0;
        if let Some(dynamic_offset) = item.dynamic_offset() {
            dynamic_offsets[offset_count] = dynamic_offset.get();
            offset_count += 1;
        }
        if let Some(skin_index) = skin_index {
            dynamic_offsets[offset_count] = skin_index.index;
            offset_count += 1;
        }
        if let Some(morph_index) = morph_index {
            dynamic_offsets[offset_count] = morph_index.index;
            offset_count += 1;
        }
        pass.set_bind_group(I, bind_group, &dynamic_offsets[0..offset_count]);

        RenderCommandResult::Success
    }
}

pub struct DrawMesh;
impl<P: PhaseItem> RenderCommand<P> for DrawMesh {
    type Param = (SRes<RenderAssets<Mesh>>, SRes<RenderMeshInstances>);
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();
    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: (),
        (meshes, mesh_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let mesh_instances = mesh_instances.into_inner();

        let Some(mesh_instance) = mesh_instances.get(&item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let Some(gpu_mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

        let batch_range = item.batch_range();
        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            &(batch_range.start as i32).to_le_bytes(),
        );
        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, batch_range.clone());
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_mesh.vertex_count, batch_range.clone());
            }
        }
        RenderCommandResult::Success
    }
}

#[cfg(test)]
mod tests {
    use super::MeshPipelineKey;
    #[test]
    fn mesh_key_msaa_samples() {
        for i in [1, 2, 4, 8, 16, 32, 64, 128] {
            assert_eq!(MeshPipelineKey::from_msaa_samples(i).msaa_samples(), i);
        }
    }
}
