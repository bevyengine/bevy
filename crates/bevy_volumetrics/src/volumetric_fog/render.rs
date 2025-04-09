//! Rendering of fog volumes.

use core::array;

use bevy_asset::AssetId;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    query::QueryItem,
    resource::Resource,
    system::lifetimeless::Read,
    world::{FromWorld, World},
};
use bevy_image::{BevyDefault, Image};
use bevy_math::{Mat4, Vec3, Vec4};
use bevy_render::render_graph::RenderLabel;
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
        DynamicBindGroupEntries, DynamicUniformBuffer, Face, FragmentState, LoadOp,
        MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
        ShaderType, SpecializedRenderPipeline, StoreOp, TextureFormat, TextureSampleType,
        VertexState,
    },
    renderer::{RenderContext, RenderDevice},
    texture::GpuImage,
    view::{Msaa, ViewDepthTexture, ViewTarget, ViewUniformOffset},
};
use bevy_render_3d::{
    MeshPipelineViewLayoutKey, MeshPipelineViewLayouts, MeshViewBindGroup,
    ViewEnvironmentMapUniformOffset, ViewFogUniformOffset, ViewLightProbesUniformOffset,
    ViewLightsUniformOffset, ViewScreenSpaceReflectionsUniformOffset,
};
use bevy_utils::prelude::default;

use bitflags::bitflags;

use crate::volumetric_fog::plugin::{CUBE_MESH, PLANE_MESH, VOLUMETRIC_FOG_HANDLE};

bitflags! {
    /// Flags that describe the bind group layout used to render volumetric fog.
    #[derive(Clone, Copy, PartialEq)]
    pub struct VolumetricFogBindGroupLayoutKey: u8 {
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
    pub struct VolumetricFogPipelineKeyFlags: u8 {
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

/// Render graph nodes specific to 3D PBR rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum NodeVolumetric {
    /// Label for the volumetric lighting pass.
    VolumetricFog,
}

/// The GPU pipeline for the volumetric fog postprocessing effect.
#[derive(Resource)]
pub struct VolumetricFogPipeline {
    /// A reference to the shared set of mesh pipeline view layouts.
    mesh_view_layouts: MeshPipelineViewLayouts,

    /// All bind group layouts.
    ///
    /// Since there aren't too many of these, we precompile them all.
    volumetric_view_bind_group_layouts: [BindGroupLayout; VOLUMETRIC_FOG_BIND_GROUP_LAYOUT_COUNT],
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
    pub mesh_pipeline_view_key: MeshPipelineViewLayoutKey,

    /// The vertex buffer layout of the primitive.
    ///
    /// Both planes (used when the camera is inside the fog volume) and cubes
    /// (used when the camera is outside the fog volume) use identical vertex
    /// buffer layouts, so we only need one of them.
    pub vertex_buffer_layout: MeshVertexBufferLayoutRef,

    /// Flags that specify features on the pipeline key.
    pub flags: VolumetricFogPipelineKeyFlags,
}

/// The same as [`VolumetricFog`](super::VolumetricFog) and [`FogVolume`](super::FogVolume), but formatted for
/// the GPU.
///
/// See the documentation of those structures for more information on these
/// fields.
#[derive(ShaderType)]
pub struct VolumetricFogUniform {
    pub clip_from_local: Mat4,

    /// The transform from world space to 3D density texture UVW space.
    pub uvw_from_world: Mat4,

    /// View-space plane equations of the far faces of the fog volume cuboid.
    ///
    /// The vector takes the form V = (N, -N⋅Q), where N is the normal of the
    /// plane and Q is any point in it, in view space. The equation of the plane
    /// for homogeneous point P = (Px, Py, Pz, Pw) is V⋅P = 0.
    pub far_planes: [Vec4; 3],

    pub fog_color: Vec3,
    pub light_tint: Vec3,
    pub ambient_color: Vec3,
    pub ambient_intensity: f32,
    pub step_count: u32,

    /// The radius of a sphere that bounds the fog volume in view space.
    pub bounding_radius: f32,

    pub absorption: f32,
    pub scattering: f32,
    pub density: f32,
    pub density_texture_offset: Vec3,
    pub scattering_asymmetry: f32,
    pub light_intensity: f32,
    pub jitter_strength: f32,
}

/// Specifies the offset within the [`VolumetricFogUniformBuffer`] of the
/// [`VolumetricFogUniform`] for a specific view.
#[derive(Component, Deref, DerefMut)]
pub struct ViewVolumetricFog(pub Vec<ViewFogVolume>);

/// Information that the render world needs to maintain about each fog volume.
pub struct ViewFogVolume {
    /// The 3D voxel density texture for this volume, if present.
    pub density_texture: Option<AssetId<Image>>,
    /// The offset of this view's [`VolumetricFogUniform`] structure within the
    /// [`VolumetricFogUniformBuffer`].
    pub uniform_buffer_offset: u32,
    /// True if the camera is outside the fog volume; false if it's inside the
    /// fog volume.
    pub exterior: bool,
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
        }
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
        ): QueryItem<'w, Self::ViewQuery>,
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

        let render_meshes = world.resource::<RenderAssets<RenderMesh>>();

        for view_fog_volume in view_fog_volumes.iter() {
            // If the camera is outside the fog volume, pick the cube mesh;
            // otherwise, pick the plane mesh. In the latter case we'll be
            // effectively rendering a full-screen quad.
            let mesh_handle = if view_fog_volume.exterior {
                CUBE_MESH.clone()
            } else {
                PLANE_MESH.clone()
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
                &view_bind_group.value,
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
        let mesh_view_layout = self
            .mesh_view_layouts
            .get_view_layout(key.mesh_pipeline_view_key);

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

        RenderPipelineDescriptor {
            label: Some("volumetric lighting pipeline".into()),
            layout: vec![mesh_view_layout.clone(), volumetric_view_bind_group_layout],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: VOLUMETRIC_FOG_HANDLE,
                shader_defs: shader_defs.clone(),
                entry_point: "vertex".into(),
                buffers: vec![vertex_format],
            },
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: VOLUMETRIC_FOG_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
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
            }),
            zero_initialize_workgroup_memory: false,
        }
    }
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
