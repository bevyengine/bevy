//! Rendering of fog volumes.

use core::array;

use bevy_asset::{load_embedded_asset, AssetId, Handle};
use bevy_color::ColorToComponents as _;
use bevy_core_pipeline::{
    core_3d::Camera3d,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryItem, With},
    resource::Resource,
    system::{lifetimeless::Read, Commands, Local, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::{BevyDefault, Image};
use bevy_math::{vec4, Mat3A, Mat4, Vec3, Vec3A, Vec4, Vec4Swizzles as _};
use bevy_render::{
    mesh::{
        allocator::MeshAllocator, Mesh, MeshVertexBufferLayoutRef, RenderMesh, RenderMeshBufferInfo,
    },
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        binding_types::{
            sampler, texture_3d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer,
        },
        BindGroupLayout, BindGroupLayoutEntries, BindingResource, BlendComponent, BlendFactor,
        BlendOperation, BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites,
        DynamicBindGroupEntries, DynamicUniformBuffer, Face, FragmentState, LoadOp, Operations,
        PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, SamplerBindingType, Shader, ShaderStages, ShaderType,
        SpecializedRenderPipeline, SpecializedRenderPipelines, StoreOp, TextureFormat,
        TextureSampleType, TextureUsages, VertexState,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    texture::GpuImage,
    view::{ExtractedView, Msaa, ViewDepthTexture, ViewTarget, ViewUniformOffset},
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::prelude::default;
use bitflags::bitflags;

use crate::{
    FogVolume, MeshPipelineViewLayoutKey, MeshPipelineViewLayouts, MeshViewBindGroup,
    ViewEnvironmentMapUniformOffset, ViewFogUniformOffset, ViewLightProbesUniformOffset,
    ViewLightsUniformOffset, ViewScreenSpaceReflectionsUniformOffset, VolumetricFog,
    VolumetricLight,
};

use super::FogAssets;

bitflags! {
    /// Flags that describe the bind group layout used to render volumetric fog.
    #[derive(Clone, Copy, PartialEq)]
    struct VolumetricFogBindGroupLayoutKey: u8 {
        /// The framebuffer is multisampled.
        const MULTISAMPLED = 0x1;
        /// The volumetric fog has a 3D voxel density texture.
        const DENSITY_TEXTURE = 0x2;
    }
}

bitflags! {
    /// Flags that describe the rasterization pipeline used to render volumetric
    /// fog.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct VolumetricFogPipelineKeyFlags: u8 {
        /// The view's color format has high dynamic range.
        const HDR = 0x1;
        /// The volumetric fog has a 3D voxel density texture.
        const DENSITY_TEXTURE = 0x2;
    }
}

/// The total number of bind group layouts.
///
/// This is the total number of combinations of all
/// [`VolumetricFogBindGroupLayoutKey`] flags.
const VOLUMETRIC_FOG_BIND_GROUP_LAYOUT_COUNT: usize =
    VolumetricFogBindGroupLayoutKey::all().bits() as usize + 1;

/// A matrix that converts from local 1×1×1 space to UVW 3D density texture
/// space.
static UVW_FROM_LOCAL: Mat4 = Mat4::from_cols(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, 1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.5, 0.5, 0.5, 1.0),
);

/// The GPU pipeline for the volumetric fog postprocessing effect.
#[derive(Resource)]
pub struct VolumetricFogPipeline {
    /// A reference to the shared set of mesh pipeline view layouts.
    mesh_view_layouts: MeshPipelineViewLayouts,

    /// All bind group layouts.
    ///
    /// Since there aren't too many of these, we precompile them all.
    volumetric_view_bind_group_layouts: [BindGroupLayout; VOLUMETRIC_FOG_BIND_GROUP_LAYOUT_COUNT],

    // The shader asset handle.
    shader: Handle<Shader>,
}

/// The two render pipelines that we use for fog volumes: one for when a 3D
/// density texture is present and one for when it isn't.
#[derive(Component)]
pub struct ViewVolumetricFogPipelines {
    /// The render pipeline that we use when no density texture is present, and
    /// the density distribution is uniform.
    pub textureless: CachedRenderPipelineId,
    /// The render pipeline that we use when a density texture is present.
    pub textured: CachedRenderPipelineId,
}

/// The node in the render graph, part of the postprocessing stack, that
/// implements volumetric fog.
#[derive(Default)]
pub struct VolumetricFogNode;

/// Identifies a single specialization of the volumetric fog shader.
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct VolumetricFogPipelineKey {
    /// The layout of the view, which is needed for the raymarching.
    mesh_pipeline_view_key: MeshPipelineViewLayoutKey,

    /// The vertex buffer layout of the primitive.
    ///
    /// Both planes (used when the camera is inside the fog volume) and cubes
    /// (used when the camera is outside the fog volume) use identical vertex
    /// buffer layouts, so we only need one of them.
    vertex_buffer_layout: MeshVertexBufferLayoutRef,

    /// Flags that specify features on the pipeline key.
    flags: VolumetricFogPipelineKeyFlags,
}

/// The same as [`VolumetricFog`] and [`FogVolume`], but formatted for
/// the GPU.
///
/// See the documentation of those structures for more information on these
/// fields.
#[derive(ShaderType)]
pub struct VolumetricFogUniform {
    clip_from_local: Mat4,

    /// The transform from world space to 3D density texture UVW space.
    uvw_from_world: Mat4,

    /// View-space plane equations of the far faces of the fog volume cuboid.
    ///
    /// The vector takes the form V = (N, -N⋅Q), where N is the normal of the
    /// plane and Q is any point in it, in view space. The equation of the plane
    /// for homogeneous point P = (Px, Py, Pz, Pw) is V⋅P = 0.
    far_planes: [Vec4; 3],

    fog_color: Vec3,
    light_tint: Vec3,
    ambient_color: Vec3,
    ambient_intensity: f32,
    step_count: u32,

    /// The radius of a sphere that bounds the fog volume in view space.
    bounding_radius: f32,

    absorption: f32,
    scattering: f32,
    density: f32,
    density_texture_offset: Vec3,
    scattering_asymmetry: f32,
    light_intensity: f32,
    jitter_strength: f32,
}

/// Specifies the offset within the [`VolumetricFogUniformBuffer`] of the
/// [`VolumetricFogUniform`] for a specific view.
#[derive(Component, Deref, DerefMut)]
pub struct ViewVolumetricFog(Vec<ViewFogVolume>);

/// Information that the render world needs to maintain about each fog volume.
pub struct ViewFogVolume {
    /// The 3D voxel density texture for this volume, if present.
    density_texture: Option<AssetId<Image>>,
    /// The offset of this view's [`VolumetricFogUniform`] structure within the
    /// [`VolumetricFogUniformBuffer`].
    uniform_buffer_offset: u32,
    /// True if the camera is outside the fog volume; false if it's inside the
    /// fog volume.
    exterior: bool,
}

/// The GPU buffer that stores the [`VolumetricFogUniform`] data.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct VolumetricFogUniformBuffer(pub DynamicUniformBuffer<VolumetricFogUniform>);

impl FromWorld for VolumetricFogPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let mesh_view_layouts = world.resource::<MeshPipelineViewLayouts>();

        // Create the bind group layout entries common to all bind group
        // layouts.
        let base_bind_group_layout_entries = &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            // `volumetric_fog`
            uniform_buffer::<VolumetricFogUniform>(true),
        );

        // For every combination of `VolumetricFogBindGroupLayoutKey` bits,
        // create a bind group layout.
        let bind_group_layouts = array::from_fn(|bits| {
            let flags = VolumetricFogBindGroupLayoutKey::from_bits_retain(bits as u8);

            let mut bind_group_layout_entries = base_bind_group_layout_entries.to_vec();

            // `depth_texture`
            bind_group_layout_entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                ((
                    1,
                    if flags.contains(VolumetricFogBindGroupLayoutKey::MULTISAMPLED) {
                        texture_depth_2d_multisampled()
                    } else {
                        texture_depth_2d()
                    },
                ),),
            ));

            // `density_texture` and `density_sampler`
            if flags.contains(VolumetricFogBindGroupLayoutKey::DENSITY_TEXTURE) {
                bind_group_layout_entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (2, texture_3d(TextureSampleType::Float { filterable: true })),
                        (3, sampler(SamplerBindingType::Filtering)),
                    ),
                ));
            }

            // Create the bind group layout.
            let description = flags.bind_group_layout_description();
            render_device.create_bind_group_layout(&*description, &bind_group_layout_entries)
        });

        VolumetricFogPipeline {
            mesh_view_layouts: mesh_view_layouts.clone(),
            volumetric_view_bind_group_layouts: bind_group_layouts,
            shader: load_embedded_asset!(world, "volumetric_fog.wgsl"),
        }
    }
}

/// Extracts [`VolumetricFog`], [`FogVolume`], and [`VolumetricLight`]s
/// from the main world to the render world.
pub fn extract_volumetric_fog(
    mut commands: Commands,
    view_targets: Extract<Query<(RenderEntity, &VolumetricFog)>>,
    fog_volumes: Extract<Query<(RenderEntity, &FogVolume, &GlobalTransform)>>,
    volumetric_lights: Extract<Query<(RenderEntity, &VolumetricLight)>>,
) {
    if volumetric_lights.is_empty() {
        // TODO: needs better way to handle clean up in render world
        for (entity, ..) in view_targets.iter() {
            commands
                .entity(entity)
                .remove::<(VolumetricFog, ViewVolumetricFogPipelines, ViewVolumetricFog)>();
        }
        for (entity, ..) in fog_volumes.iter() {
            commands.entity(entity).remove::<FogVolume>();
        }
        return;
    }

    for (entity, volumetric_fog) in view_targets.iter() {
        commands
            .get_entity(entity)
            .expect("Volumetric fog entity wasn't synced.")
            .insert(*volumetric_fog);
    }

    for (entity, fog_volume, fog_transform) in fog_volumes.iter() {
        commands
            .get_entity(entity)
            .expect("Fog volume entity wasn't synced.")
            .insert((*fog_volume).clone())
            .insert(*fog_transform);
    }

    for (entity, volumetric_light) in volumetric_lights.iter() {
        commands
            .get_entity(entity)
            .expect("Volumetric light entity wasn't synced.")
            .insert(*volumetric_light);
    }
}

impl ViewNode for VolumetricFogNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<ViewDepthTexture>,
        Read<ViewVolumetricFogPipelines>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<ViewFogUniformOffset>,
        Read<ViewLightProbesUniformOffset>,
        Read<ViewVolumetricFog>,
        Read<MeshViewBindGroup>,
        Read<ViewScreenSpaceReflectionsUniformOffset>,
        Read<Msaa>,
        Read<ViewEnvironmentMapUniformOffset>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            view_target,
            view_depth_texture,
            view_volumetric_lighting_pipelines,
            view_uniform_offset,
            view_lights_offset,
            view_fog_offset,
            view_light_probes_offset,
            view_fog_volumes,
            view_bind_group,
            view_ssr_offset,
            msaa,
            view_environment_map_offset,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let volumetric_lighting_pipeline = world.resource::<VolumetricFogPipeline>();
        let volumetric_lighting_uniform_buffers = world.resource::<VolumetricFogUniformBuffer>();
        let image_assets = world.resource::<RenderAssets<GpuImage>>();
        let mesh_allocator = world.resource::<MeshAllocator>();

        // Fetch the uniform buffer and binding.
        let (
            Some(textureless_pipeline),
            Some(textured_pipeline),
            Some(volumetric_lighting_uniform_buffer_binding),
        ) = (
            pipeline_cache.get_render_pipeline(view_volumetric_lighting_pipelines.textureless),
            pipeline_cache.get_render_pipeline(view_volumetric_lighting_pipelines.textured),
            volumetric_lighting_uniform_buffers.binding(),
        )
        else {
            return Ok(());
        };

        let fog_assets = world.resource::<FogAssets>();
        let render_meshes = world.resource::<RenderAssets<RenderMesh>>();

        for view_fog_volume in view_fog_volumes.iter() {
            // If the camera is outside the fog volume, pick the cube mesh;
            // otherwise, pick the plane mesh. In the latter case we'll be
            // effectively rendering a full-screen quad.
            let mesh_handle = if view_fog_volume.exterior {
                fog_assets.cube_mesh.clone()
            } else {
                fog_assets.plane_mesh.clone()
            };

            let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(&mesh_handle.id())
            else {
                continue;
            };

            let density_image = view_fog_volume
                .density_texture
                .and_then(|density_texture| image_assets.get(density_texture));

            // Pick the right pipeline, depending on whether a density texture
            // is present or not.
            let pipeline = if density_image.is_some() {
                textured_pipeline
            } else {
                textureless_pipeline
            };

            // This should always succeed, but if the asset was unloaded don't
            // panic.
            let Some(render_mesh) = render_meshes.get(&mesh_handle) else {
                return Ok(());
            };

            // Create the bind group for the view.
            //
            // TODO: Cache this.

            let mut bind_group_layout_key = VolumetricFogBindGroupLayoutKey::empty();
            bind_group_layout_key.set(
                VolumetricFogBindGroupLayoutKey::MULTISAMPLED,
                !matches!(*msaa, Msaa::Off),
            );

            // Create the bind group entries. The ones relating to the density
            // texture will only be filled in if that texture is present.
            let mut bind_group_entries = DynamicBindGroupEntries::sequential((
                volumetric_lighting_uniform_buffer_binding.clone(),
                BindingResource::TextureView(view_depth_texture.view()),
            ));
            if let Some(density_image) = density_image {
                bind_group_layout_key.insert(VolumetricFogBindGroupLayoutKey::DENSITY_TEXTURE);
                bind_group_entries = bind_group_entries.extend_sequential((
                    BindingResource::TextureView(&density_image.texture_view),
                    BindingResource::Sampler(&density_image.sampler),
                ));
            }

            let volumetric_view_bind_group_layout = &volumetric_lighting_pipeline
                .volumetric_view_bind_group_layouts[bind_group_layout_key.bits() as usize];

            let volumetric_view_bind_group = render_context.render_device().create_bind_group(
                None,
                volumetric_view_bind_group_layout,
                &bind_group_entries,
            );

            let render_pass_descriptor = RenderPassDescriptor {
                label: Some("volumetric lighting pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: view_target.main_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&render_pass_descriptor);

            render_pass.set_vertex_buffer(0, *vertex_buffer_slice.buffer.slice(..));
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(
                0,
                &view_bind_group.main,
                &[
                    view_uniform_offset.offset,
                    view_lights_offset.offset,
                    view_fog_offset.offset,
                    **view_light_probes_offset,
                    **view_ssr_offset,
                    **view_environment_map_offset,
                ],
            );
            render_pass.set_bind_group(
                1,
                &volumetric_view_bind_group,
                &[view_fog_volume.uniform_buffer_offset],
            );

            // Draw elements or arrays, as appropriate.
            match &render_mesh.buffer_info {
                RenderMeshBufferInfo::Indexed {
                    index_format,
                    count,
                } => {
                    let Some(index_buffer_slice) =
                        mesh_allocator.mesh_index_slice(&mesh_handle.id())
                    else {
                        continue;
                    };

                    render_pass
                        .set_index_buffer(*index_buffer_slice.buffer.slice(..), *index_format);
                    render_pass.draw_indexed(
                        index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                        vertex_buffer_slice.range.start as i32,
                        0..1,
                    );
                }
                RenderMeshBufferInfo::NonIndexed => {
                    render_pass.draw(vertex_buffer_slice.range, 0..1);
                }
            }
        }

        Ok(())
    }
}

impl SpecializedRenderPipeline for VolumetricFogPipeline {
    type Key = VolumetricFogPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // We always use hardware 2x2 filtering for sampling the shadow map; the
        // more accurate versions with percentage-closer filtering aren't worth
        // the overhead.
        let mut shader_defs = vec!["SHADOW_FILTER_METHOD_HARDWARE_2X2".into()];

        // We need a separate layout for MSAA and non-MSAA, as well as one for
        // the presence or absence of the density texture.
        let mut bind_group_layout_key = VolumetricFogBindGroupLayoutKey::empty();
        bind_group_layout_key.set(
            VolumetricFogBindGroupLayoutKey::MULTISAMPLED,
            key.mesh_pipeline_view_key
                .contains(MeshPipelineViewLayoutKey::MULTISAMPLED),
        );
        bind_group_layout_key.set(
            VolumetricFogBindGroupLayoutKey::DENSITY_TEXTURE,
            key.flags
                .contains(VolumetricFogPipelineKeyFlags::DENSITY_TEXTURE),
        );

        let volumetric_view_bind_group_layout =
            self.volumetric_view_bind_group_layouts[bind_group_layout_key.bits() as usize].clone();

        // Both the cube and plane have the same vertex layout, so we don't need
        // to distinguish between the two.
        let vertex_format = key
            .vertex_buffer_layout
            .0
            .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])
            .expect("Failed to get vertex layout for volumetric fog hull");

        if key
            .mesh_pipeline_view_key
            .contains(MeshPipelineViewLayoutKey::MULTISAMPLED)
        {
            shader_defs.push("MULTISAMPLED".into());
        }

        if key
            .flags
            .contains(VolumetricFogPipelineKeyFlags::DENSITY_TEXTURE)
        {
            shader_defs.push("DENSITY_TEXTURE".into());
        }

        let layout = self
            .mesh_view_layouts
            .get_view_layout(key.mesh_pipeline_view_key);
        let layout = vec![
            layout.main_layout.clone(),
            volumetric_view_bind_group_layout.clone(),
        ];

        RenderPipelineDescriptor {
            label: Some("volumetric lighting pipeline".into()),
            layout,
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_format],
                ..default()
            },
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: if key.flags.contains(VolumetricFogPipelineKeyFlags::HDR) {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    // Blend on top of what's already in the framebuffer. Doing
                    // the alpha blending with the hardware blender allows us to
                    // avoid having to use intermediate render targets.
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

/// Specializes volumetric fog pipelines for all views with that effect enabled.
pub fn prepare_volumetric_fog_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<VolumetricFogPipeline>>,
    volumetric_lighting_pipeline: Res<VolumetricFogPipeline>,
    fog_assets: Res<FogAssets>,
    view_targets: Query<
        (
            Entity,
            &ExtractedView,
            &Msaa,
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
        With<VolumetricFog>,
    >,
    meshes: Res<RenderAssets<RenderMesh>>,
) {
    let Some(plane_mesh) = meshes.get(&fog_assets.plane_mesh) else {
        // There's an off chance that the mesh won't be prepared yet if `RenderAssetBytesPerFrame` limiting is in use.
        return;
    };

    for (
        entity,
        view,
        msaa,
        normal_prepass,
        depth_prepass,
        motion_vector_prepass,
        deferred_prepass,
    ) in view_targets.iter()
    {
        // Create a mesh pipeline view layout key corresponding to the view.
        let mut mesh_pipeline_view_key = MeshPipelineViewLayoutKey::from(*msaa);
        mesh_pipeline_view_key.set(MeshPipelineViewLayoutKey::NORMAL_PREPASS, normal_prepass);
        mesh_pipeline_view_key.set(MeshPipelineViewLayoutKey::DEPTH_PREPASS, depth_prepass);
        mesh_pipeline_view_key.set(
            MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS,
            motion_vector_prepass,
        );
        mesh_pipeline_view_key.set(
            MeshPipelineViewLayoutKey::DEFERRED_PREPASS,
            deferred_prepass,
        );

        let mut textureless_flags = VolumetricFogPipelineKeyFlags::empty();
        textureless_flags.set(VolumetricFogPipelineKeyFlags::HDR, view.hdr);

        // Specialize the pipeline.
        let textureless_pipeline_key = VolumetricFogPipelineKey {
            mesh_pipeline_view_key,
            vertex_buffer_layout: plane_mesh.layout.clone(),
            flags: textureless_flags,
        };
        let textureless_pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &volumetric_lighting_pipeline,
            textureless_pipeline_key.clone(),
        );
        let textured_pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &volumetric_lighting_pipeline,
            VolumetricFogPipelineKey {
                flags: textureless_pipeline_key.flags
                    | VolumetricFogPipelineKeyFlags::DENSITY_TEXTURE,
                ..textureless_pipeline_key
            },
        );

        commands.entity(entity).insert(ViewVolumetricFogPipelines {
            textureless: textureless_pipeline_id,
            textured: textured_pipeline_id,
        });
    }
}

/// A system that converts [`VolumetricFog`] into [`VolumetricFogUniform`]s.
pub fn prepare_volumetric_fog_uniforms(
    mut commands: Commands,
    mut volumetric_lighting_uniform_buffer: ResMut<VolumetricFogUniformBuffer>,
    view_targets: Query<(Entity, &ExtractedView, &VolumetricFog)>,
    fog_volumes: Query<(Entity, &FogVolume, &GlobalTransform)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut local_from_world_matrices: Local<Vec<Mat4>>,
) {
    // Do this up front to avoid O(n^2) matrix inversion.
    local_from_world_matrices.clear();
    for (_, _, fog_transform) in fog_volumes.iter() {
        local_from_world_matrices.push(fog_transform.to_matrix().inverse());
    }

    let uniform_count = view_targets.iter().len() * local_from_world_matrices.len();

    let Some(mut writer) =
        volumetric_lighting_uniform_buffer.get_writer(uniform_count, &render_device, &render_queue)
    else {
        return;
    };

    for (view_entity, extracted_view, volumetric_fog) in view_targets.iter() {
        let world_from_view = extracted_view.world_from_view.to_matrix();

        let mut view_fog_volumes = vec![];

        for ((_, fog_volume, _), local_from_world) in
            fog_volumes.iter().zip(local_from_world_matrices.iter())
        {
            // Calculate the transforms to and from 1×1×1 local space.
            let local_from_view = *local_from_world * world_from_view;
            let view_from_local = local_from_view.inverse();

            // Determine whether the camera is inside or outside the volume, and
            // calculate the clip space transform.
            let interior = camera_is_inside_fog_volume(&local_from_view);
            let hull_clip_from_local = calculate_fog_volume_clip_from_local_transforms(
                interior,
                &extracted_view.clip_from_view,
                &view_from_local,
            );

            // Calculate the radius of the sphere that bounds the fog volume.
            let bounding_radius = (Mat3A::from_mat4(view_from_local) * Vec3A::splat(0.5)).length();

            // Write out our uniform.
            let uniform_buffer_offset = writer.write(&VolumetricFogUniform {
                clip_from_local: hull_clip_from_local,
                uvw_from_world: UVW_FROM_LOCAL * *local_from_world,
                far_planes: get_far_planes(&view_from_local),
                fog_color: fog_volume.fog_color.to_linear().to_vec3(),
                light_tint: fog_volume.light_tint.to_linear().to_vec3(),
                ambient_color: volumetric_fog.ambient_color.to_linear().to_vec3(),
                ambient_intensity: volumetric_fog.ambient_intensity,
                step_count: volumetric_fog.step_count,
                bounding_radius,
                absorption: fog_volume.absorption,
                scattering: fog_volume.scattering,
                density: fog_volume.density_factor,
                density_texture_offset: fog_volume.density_texture_offset,
                scattering_asymmetry: fog_volume.scattering_asymmetry,
                light_intensity: fog_volume.light_intensity,
                jitter_strength: volumetric_fog.jitter,
            });

            view_fog_volumes.push(ViewFogVolume {
                uniform_buffer_offset,
                exterior: !interior,
                density_texture: fog_volume.density_texture.as_ref().map(Handle::id),
            });
        }

        commands
            .entity(view_entity)
            .insert(ViewVolumetricFog(view_fog_volumes));
    }
}

/// A system that marks all view depth textures as readable in shaders.
///
/// The volumetric lighting pass needs to do this, and it doesn't happen by
/// default.
pub fn prepare_view_depth_textures_for_volumetric_fog(
    mut view_targets: Query<&mut Camera3d>,
    fog_volumes: Query<&VolumetricFog>,
) {
    if fog_volumes.is_empty() {
        return;
    }

    for mut camera in view_targets.iter_mut() {
        camera.depth_texture_usages.0 |= TextureUsages::TEXTURE_BINDING.bits();
    }
}

fn get_far_planes(view_from_local: &Mat4) -> [Vec4; 3] {
    let (mut far_planes, mut next_index) = ([Vec4::ZERO; 3], 0);
    let view_from_normal_local = Mat3A::from_mat4(*view_from_local);

    for &local_normal in &[
        Vec3A::X,
        Vec3A::NEG_X,
        Vec3A::Y,
        Vec3A::NEG_Y,
        Vec3A::Z,
        Vec3A::NEG_Z,
    ] {
        let view_normal = (view_from_normal_local * local_normal).normalize_or_zero();
        if view_normal.z <= 0.0 {
            continue;
        }

        let view_position = *view_from_local * (-local_normal * 0.5).extend(1.0);
        let plane_coords = view_normal.extend(-view_normal.dot(view_position.xyz().into()));

        far_planes[next_index] = plane_coords;
        next_index += 1;
        if next_index == far_planes.len() {
            continue;
        }
    }

    far_planes
}

impl VolumetricFogBindGroupLayoutKey {
    /// Creates an appropriate debug description for the bind group layout with
    /// these flags.
    fn bind_group_layout_description(&self) -> String {
        if self.is_empty() {
            return "volumetric lighting view bind group layout".to_owned();
        }

        format!(
            "volumetric lighting view bind group layout ({})",
            self.iter()
                .filter_map(|flag| {
                    if flag == VolumetricFogBindGroupLayoutKey::DENSITY_TEXTURE {
                        Some("density texture")
                    } else if flag == VolumetricFogBindGroupLayoutKey::MULTISAMPLED {
                        Some("multisampled")
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// Given the transform from the view to the 1×1×1 cube in local fog volume
/// space, returns true if the camera is inside the volume.
fn camera_is_inside_fog_volume(local_from_view: &Mat4) -> bool {
    Vec3A::from(local_from_view.col(3).xyz())
        .abs()
        .cmple(Vec3A::splat(0.5))
        .all()
}

/// Given the local transforms, returns the matrix that transforms model space
/// to clip space.
fn calculate_fog_volume_clip_from_local_transforms(
    interior: bool,
    clip_from_view: &Mat4,
    view_from_local: &Mat4,
) -> Mat4 {
    if !interior {
        return *clip_from_view * *view_from_local;
    }

    // If the camera is inside the fog volume, then we'll be rendering a full
    // screen quad. The shader will start its raymarch at the fragment depth
    // value, however, so we need to make sure that the depth of the full screen
    // quad is at the near clip plane `z_near`.
    let z_near = clip_from_view.w_axis[2];
    Mat4::from_cols(
        vec4(z_near, 0.0, 0.0, 0.0),
        vec4(0.0, z_near, 0.0, 0.0),
        vec4(0.0, 0.0, 0.0, 0.0),
        vec4(0.0, 0.0, z_near, z_near),
    )
}
