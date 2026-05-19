//! Rendering of fog volumes.

use core::array;

use bevy_asset::{load_embedded_asset, AssetId, AssetServer, Handle};
use bevy_camera::Camera3d;
use bevy_color::ColorToComponents as _;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    resource::Resource,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_image::Image;
use bevy_light::{FogVolume, VolumetricFog, VolumetricLight};
use bevy_math::{vec4, Affine3A, Mat4, Vec3, Vec3A, Vec4};
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_render::{
    mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{
            sampler, texture_3d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer,
        },
        BindGroupLayoutDescriptor, BindGroupLayoutEntries, BindingResource, BlendComponent,
        BlendFactor, BlendOperation, BlendState, CachedRenderPipelineId, ColorTargetState,
        ColorWrites, DynamicBindGroupEntries, DynamicUniformBuffer, Face, FragmentState, LoadOp,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, SamplerBindingType, ShaderStages, ShaderType,
        SpecializedRenderPipeline, SpecializedRenderPipelines, StoreOp, TextureFormat,
        TextureSampleType, TextureUsages, VertexState,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
    sync_world::RenderEntity,
    texture::GpuImage,
    view::{ExtractedView, Msaa, ViewDepthTexture, ViewTarget},
    Extract,
};
use bevy_shader::Shader;
use bevy_transform::components::GlobalTransform;
use bevy_utils::prelude::default;
use bitflags::bitflags;

use crate::{MeshPipelineViewLayoutKey, MeshPipelineViewLayouts, MeshViewBindGroup, ViewKeyCache};

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
    volumetric_view_bind_group_layouts:
        [BindGroupLayoutDescriptor; VOLUMETRIC_FOG_BIND_GROUP_LAYOUT_COUNT],

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

    /// Texture format of the view target
    target_format: TextureFormat,

    /// The volumetric fog has a 3D voxel density texture.
    has_density_texture: bool,
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
    boundary_fade: f32,
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

pub fn init_volumetric_fog_pipeline(
    mut commands: Commands,
    mesh_view_layouts: Res<MeshPipelineViewLayouts>,
    asset_server: Res<AssetServer>,
) {
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
        BindGroupLayoutDescriptor::new(description, &bind_group_layout_entries)
    });

    commands.insert_resource(VolumetricFogPipeline {
        mesh_view_layouts: mesh_view_layouts.clone(),
        volumetric_view_bind_group_layouts: bind_group_layouts,
        shader: load_embedded_asset!(asset_server.as_ref(), "volumetric_fog.wgsl"),
    });
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

pub fn volumetric_fog(
    view: ViewQuery<(
        &ViewTarget,
        &ViewDepthTexture,
        &ViewVolumetricFogPipelines,
        &ViewVolumetricFog,
        &MeshViewBindGroup,
        &Msaa,
    )>,
    pipeline_cache: Res<PipelineCache>,
    volumetric_lighting_pipeline: Res<VolumetricFogPipeline>,
    volumetric_lighting_uniform_buffers: Res<VolumetricFogUniformBuffer>,
    image_assets: Res<RenderAssets<GpuImage>>,
    mesh_allocator: Res<MeshAllocator>,
    fog_assets: Res<FogAssets>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    mut ctx: RenderContext,
) {
    let (
        view_target,
        view_depth_texture,
        view_volumetric_lighting_pipelines,
        view_fog_volumes,
        view_bind_group,
        msaa,
    ) = view.into_inner();

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
        return;
    };

    let command_encoder = ctx.command_encoder();
    command_encoder.push_debug_group("volumetric_lighting");

    for view_fog_volume in view_fog_volumes.iter() {
        // If the camera is outside the fog volume, pick the cube mesh;
        // otherwise, pick the plane mesh. In the latter case we'll be
        // effectively rendering a full-screen quad.
        let mesh_handle = if view_fog_volume.exterior {
            fog_assets.cube_mesh.clone()
        } else {
            fog_assets.plane_mesh.clone()
        };

        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(&mesh_handle.id()) else {
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
            return;
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

        let volumetric_view_bind_group = ctx.render_device().create_bind_group(
            None,
            &pipeline_cache.get_bind_group_layout(volumetric_view_bind_group_layout),
            &bind_group_entries,
        );

        let render_pass_descriptor = RenderPassDescriptor {
            label: Some("volumetric lighting pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: view_target.main_texture_view(),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        };

        let command_encoder = ctx.command_encoder();
        let mut render_pass = command_encoder.begin_render_pass(&render_pass_descriptor);

        render_pass.set_vertex_buffer(0, *vertex_buffer_slice.buffer.slice(..));
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, &view_bind_group.main, &view_bind_group.main_offsets);
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
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(&mesh_handle.id())
                else {
                    continue;
                };

                render_pass.set_index_buffer(*index_buffer_slice.buffer.slice(..), *index_format);
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

    ctx.command_encoder().pop_debug_group();
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
            key.has_density_texture,
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
            .mesh_pipeline_view_key
            .contains(MeshPipelineViewLayoutKey::ATMOSPHERE)
        {
            shader_defs.push("ATMOSPHERE".into());
        }

        if key.has_density_texture {
            shader_defs.push("DENSITY_TEXTURE".into());
        }

        let layout = self
            .mesh_view_layouts
            .get_view_layout(key.mesh_pipeline_view_key);
        let layout = vec![
            layout.main_layout,
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
                    format: key.target_format,
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
    view_targets: Query<(Entity, &ExtractedView), With<VolumetricFog>>,
    meshes: Res<RenderAssets<RenderMesh>>,
    view_key_cache: Res<ViewKeyCache>,
) {
    let Some(plane_mesh) = meshes.get(&fog_assets.plane_mesh) else {
        // There's an off chance that the mesh won't be prepared yet if `RenderAssetBytesPerFrame` limiting is in use.
        return;
    };

    for (entity, view) in view_targets.iter() {
        let Some(mesh_pipeline_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        // Specialize the pipeline.
        let textureless_pipeline_key = VolumetricFogPipelineKey {
            mesh_pipeline_view_key: (*mesh_pipeline_key).into(),
            vertex_buffer_layout: plane_mesh.layout.clone(),
            target_format: view.target_format,
            has_density_texture: false,
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
                has_density_texture: true,
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
    mut local_from_world_matrices: Local<Vec<Affine3A>>,
) {
    // Do this up front to avoid O(n^2) matrix inversion.
    local_from_world_matrices.clear();
    for (_, _, fog_transform) in fog_volumes.iter() {
        local_from_world_matrices.push(fog_transform.affine().inverse());
    }

    let uniform_count = view_targets.iter().len() * local_from_world_matrices.len();

    let Some(mut writer) =
        volumetric_lighting_uniform_buffer.get_writer(uniform_count, &render_device, &render_queue)
    else {
        return;
    };

    for (view_entity, extracted_view, volumetric_fog) in view_targets.iter() {
        let world_from_view = extracted_view.world_from_view.affine();
        let mut view_fog_volumes = vec![];

        for ((_, fog_volume, _), local_from_world) in
            fog_volumes.iter().zip(local_from_world_matrices.iter())
        {
            // Calculate the transforms to and from 1×1×1 local space.
            let local_from_view = *local_from_world * world_from_view;
            let view_from_local = local_from_view.inverse();

            // Determine whether the camera is inside or outside the volume, and
            // calculate the clip space transform.
            let z_near = extracted_view.clip_from_view.w_axis[2];
            let interior = camera_is_inside_fog_volume(&local_from_view, z_near);
            let distance_to_border = fog_volume_boundary_distance(&local_from_view);
            let hull_clip_from_local = calculate_fog_volume_clip_from_local_transforms(
                interior,
                &extracted_view.clip_from_view,
                &view_from_local,
            );

            // Calculate the radius of the sphere that bounds the fog volume.
            let bounding_radius = view_from_local
                .transform_vector3a(Vec3A::splat(0.5))
                .length();

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
                boundary_fade: calculate_fog_volume_boundary_fade(
                    &local_from_view,
                    interior,
                    distance_to_border,
                ),
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

fn get_far_planes(view_from_local: &Affine3A) -> [Vec4; 3] {
    let mut far_planes = [Vec4::ZERO; 3];

    // Iterate the three axis-aligned face pairs of the unit cube.
    // For each pair, always pick exactly one face — the one whose view-space
    // normal is more front-facing (larger z component).
    for (i, &(pos_normal, neg_normal)) in [
        (Vec3A::X, Vec3A::NEG_X),
        (Vec3A::Y, Vec3A::NEG_Y),
        (Vec3A::Z, Vec3A::NEG_Z),
    ]
    .iter()
    .enumerate()
    {
        let pos_view = view_from_local
            .transform_vector3a(pos_normal)
            .normalize_or_zero();
        let neg_view = view_from_local
            .transform_vector3a(neg_normal)
            .normalize_or_zero();

        // Pick the face whose view-space normal has the larger z (more
        // front-facing).
        let (local_normal, view_normal) = if pos_view.z >= neg_view.z {
            (pos_normal, pos_view)
        } else {
            (neg_normal, neg_view)
        };

        let view_position = view_from_local.transform_point3a(local_normal * 0.5);
        far_planes[i] = view_normal.extend(-view_normal.dot(view_position));
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
/// space, returns true if the camera is inside (or within near-clip distance
/// of) the volume. Expanding by `z_near` ensures we switch to interior
/// rendering just before the near plane clips the front-face geometry, making
/// the depth continuous across the mode switch.
fn camera_is_inside_fog_volume(local_from_view: &Affine3A, z_near: f32) -> bool {
    // Use the full near-clip distance in local space as a uniform expansion.
    let near_clip_extent = (local_from_view.matrix3 * Vec3A::Z).length() * z_near;
    local_from_view
        .translation
        .abs()
        .cmple(Vec3A::splat(0.5 + near_clip_extent))
        .all()
}

fn fog_volume_boundary_distance(local_from_view: &Affine3A) -> f32 {
    0.5 - local_from_view.translation.abs().max_element()
}

/// Computes the distance from the camera to the first exit point from the
/// fog volume along the camera forward direction.
fn fog_exit_distance_along_view(local_from_view: &Affine3A) -> f32 {
    let camera_pos = local_from_view.translation;
    let camera_forward_local = (local_from_view.matrix3 * Vec3A::NEG_Z).normalize_or_zero();

    // Compute intersections with the six faces of the unit cube (±0.5).
    let mut closest_t = 1e30_f32;

    for &(axis, face) in &[
        (0, 0.5),
        (0, -0.5),
        (1, 0.5),
        (1, -0.5),
        (2, 0.5),
        (2, -0.5),
    ] {
        let denom = camera_forward_local[axis];
        if denom.abs() < 1e-6 {
            continue;
        }
        let t = (face - camera_pos[axis]) / denom;
        if t > 0.0 {
            let hit = camera_pos + camera_forward_local * t;
            let other_axes = match axis {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            if hit[other_axes.0].abs() <= 0.5 && hit[other_axes.1].abs() <= 0.5 {
                closest_t = closest_t.min(t);
            }
        }
    }

    closest_t.max(0.1)
}

/// Returns a fade factor based on proximity to the volume boundary.
fn calculate_fog_volume_boundary_fade(
    local_from_view: &Affine3A,
    interior: bool,
    distance_to_border: f32,
) -> f32 {
    if !interior {
        return 1.0;
    }

    let center_to_camera_dir_local = local_from_view.translation.normalize_or_zero();
    let camera_forward_local = (local_from_view.matrix3 * Vec3A::NEG_Z).normalize_or_zero();
    let outward_view_alignment = camera_forward_local.dot(center_to_camera_dir_local);
    let is_looking_toward_volume_center = outward_view_alignment < 0.0;
    if is_looking_toward_volume_center {
        return 1.0;
    }

    // Use directional exit distance through the volume along view direction.
    let fade_extent = fog_exit_distance_along_view(local_from_view);

    if fade_extent <= 0.0 {
        return 1.0;
    }

    let t = (distance_to_border / fade_extent).clamp(0.0, 1.0);
    let base_fade = t * t * (3.0 - 2.0 * t);

    let outward_weight = outward_view_alignment.clamp(0.0, 1.0);
    1.0 - outward_weight * (1.0 - base_fade)
}

/// Given the local transforms, returns the matrix that transforms model space
/// to clip space.
fn calculate_fog_volume_clip_from_local_transforms(
    interior: bool,
    clip_from_view: &Mat4,
    view_from_local: &Affine3A,
) -> Mat4 {
    if !interior {
        return *clip_from_view * Mat4::from(*view_from_local);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_inside_without_near_clip_expansion_matches_box_test() {
        let mut local_from_view = Affine3A::IDENTITY;
        local_from_view.translation = Vec3A::new(0.5, 0.5, 0.5);
        assert!(camera_is_inside_fog_volume(&local_from_view, 0.0));

        local_from_view.translation = Vec3A::new(0.5001, 0.0, 0.0);
        assert!(!camera_is_inside_fog_volume(&local_from_view, 0.0));
    }

    #[test]
    fn camera_inside_with_near_clip_expansion_includes_boundary_margin() {
        let mut local_from_view = Affine3A::IDENTITY;

        // For identity transform, near_clip_extent == z_near.
        local_from_view.translation = Vec3A::new(0.55, 0.0, 0.0);
        assert!(camera_is_inside_fog_volume(&local_from_view, 0.05));

        local_from_view.translation = Vec3A::new(0.56, 0.0, 0.0);
        assert!(!camera_is_inside_fog_volume(&local_from_view, 0.05));
    }

    #[test]
    fn fog_volume_boundary_distance_is_signed_distance_to_surface() {
        let mut local_from_view = Affine3A::IDENTITY;

        local_from_view.translation = Vec3A::new(0.0, 0.0, 0.0);
        assert_eq!(fog_volume_boundary_distance(&local_from_view), 0.5);

        local_from_view.translation = Vec3A::new(0.5, 0.0, 0.0);
        assert_eq!(fog_volume_boundary_distance(&local_from_view), 0.0);

        local_from_view.translation = Vec3A::new(0.6, 0.0, 0.0);
        assert!((fog_volume_boundary_distance(&local_from_view) + 0.1).abs() < 1e-6);
    }

    #[test]
    fn fog_exit_distance_along_view_hits_unit_cube_faces() {
        let mut local_from_view = Affine3A::IDENTITY;

        local_from_view.translation = Vec3A::ZERO;
        assert_eq!(fog_exit_distance_along_view(&local_from_view), 0.5);

        local_from_view.translation = Vec3A::new(0.0, 0.0, 0.25);
        assert_eq!(fog_exit_distance_along_view(&local_from_view), 0.75);
    }

    #[test]
    fn boundary_fade_is_full_outside_and_tapers_inside() {
        let mut local_from_view = Affine3A::IDENTITY;

        assert_eq!(
            calculate_fog_volume_boundary_fade(&local_from_view, false, 0.5),
            1.0
        );

        // Place camera on -Z axis so default forward (-Z) is outward.
        local_from_view.translation = Vec3A::new(0.0, 0.0, -0.4);
        assert_eq!(
            calculate_fog_volume_boundary_fade(&local_from_view, true, 0.1),
            1.0
        );

        local_from_view.translation = Vec3A::new(0.0, 0.0, -0.5);
        assert_eq!(
            calculate_fog_volume_boundary_fade(&local_from_view, true, 0.0),
            0.0
        );

        // Looking back toward the center disables the fade.
        local_from_view.translation = Vec3A::new(0.0, 0.0, 0.4);
        assert_eq!(
            calculate_fog_volume_boundary_fade(&local_from_view, true, 0.1),
            1.0
        );
    }

    #[test]
    fn get_far_planes_identity_selects_three_axis_planes() {
        let far_planes = get_far_planes(&Affine3A::IDENTITY);

        assert_eq!(far_planes[0], Vec4::new(1.0, 0.0, 0.0, -0.5));
        assert_eq!(far_planes[1], Vec4::new(0.0, 1.0, 0.0, -0.5));
        assert_eq!(far_planes[2], Vec4::new(0.0, 0.0, 1.0, -0.5));
    }
}
