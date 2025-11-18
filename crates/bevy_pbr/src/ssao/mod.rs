use crate::NodePbr;
use bevy_app::{App, SubApp};
use bevy_asset::{embedded_asset, load_embedded_asset};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures},
};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::Has,
    reflect::ReflectComponent,
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_image::ToExtents;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::{ExtractedCamera, TemporalJitter},
    extract_component::ExtractComponent,
    globals::GlobalsBuffer,
    render_graph::{IntoRenderNodeArray, RenderLabel},
    render_resource::*,
    render_task::{
        bind::{
            DynamicUniformBuffer, SampledTexture, SamplerFiltering, SamplerNonFiltering,
            StorageTextureWriteOnly, UniformBuffer,
        },
        RenderTask, RenderTaskContext,
    },
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    view::{ViewUniformOffset, ViewUniforms},
};
use bevy_shader::{load_shader_library, ShaderDefVal};
use bevy_utils::prelude::default;
use core::mem;

/// Component to apply screen space ambient occlusion to a 3d camera.
///
/// Screen space ambient occlusion (SSAO) approximates small-scale,
/// local occlusion of _indirect_ diffuse light between objects, based on what's visible on-screen.
/// SSAO does not apply to direct lighting, such as point or directional lights.
///
/// This darkens creases, e.g. on staircases, and gives nice contact shadows
/// where objects meet, giving entities a more "grounded" feel.
///
/// # Usage Notes
///
/// Requires that you add `RenderTaskPlugin<ScreenSpaceAmbientOcclusion>` to your app.
///
/// It strongly recommended that you use SSAO in conjunction with
/// TAA (`TemporalAntiAliasing`).
/// Doing so greatly reduces SSAO noise.
///
/// SSAO is not supported on `WebGL2`, and is not currently supported on `WebGPU`.
#[derive(Component, ExtractComponent, Reflect, PartialEq, Clone, Debug)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[require(DepthPrepass, NormalPrepass)]
#[doc(alias = "Ssao")]
pub struct ScreenSpaceAmbientOcclusion {
    /// Quality of the SSAO effect.
    pub quality_level: ScreenSpaceAmbientOcclusionQualityLevel,
    /// A constant estimated thickness of objects.
    ///
    /// This value is used to decide how far behind an object a ray of light needs to be in order
    /// to pass behind it. Any ray closer than that will be occluded.
    pub constant_object_thickness: f32,
}

impl Default for ScreenSpaceAmbientOcclusion {
    fn default() -> Self {
        Self {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::default(),
            constant_object_thickness: 0.25,
        }
    }
}

#[derive(Reflect, PartialEq, Eq, Hash, Clone, Copy, Default, Debug)]
#[reflect(PartialEq, Hash, Clone, Default)]
pub enum ScreenSpaceAmbientOcclusionQualityLevel {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
    Custom {
        /// Higher slice count means less noise, but worse performance.
        slice_count: u32,
        /// Samples per slice side is also tweakable, but recommended to be left at 2 or 3.
        samples_per_slice_side: u32,
    },
}

impl ScreenSpaceAmbientOcclusionQualityLevel {
    fn sample_counts(&self) -> (i32, i32) {
        match self {
            Self::Low => (1, 2),    // 4 spp (1 * (2 * 2)), plus optional temporal samples
            Self::Medium => (2, 2), // 8 spp (2 * (2 * 2)), plus optional temporal samples
            Self::High => (3, 3),   // 18 spp (3 * (3 * 2)), plus optional temporal samples
            Self::Ultra => (9, 3),  // 54 spp (9 * (3 * 2)), plus optional temporal samples
            Self::Custom {
                slice_count: slices,
                samples_per_slice_side,
            } => (*slices as i32, *samples_per_slice_side as i32),
        }
    }
}

impl RenderTask for ScreenSpaceAmbientOcclusion {
    type RenderNodeSubGraph = Core3d;

    fn render_node_label() -> impl RenderLabel {
        NodePbr::ScreenSpaceAmbientOcclusion
    }

    fn render_node_ordering() -> impl IntoRenderNodeArray {
        (
            Node3d::EndPrepasses,
            Self::render_node_label(),
            Node3d::StartMainPass,
        )
    }

    const REQUIRED_LIMITS: WgpuLimits = WgpuLimits {
        max_storage_textures_per_shader_stage: 5,
        ..WgpuLimits::downlevel_webgl2_defaults()
    };

    fn plugin_app_build(app: &mut App) {
        load_shader_library!(app, "ssao_utils.wgsl");

        embedded_asset!(app, "preprocess_depth.wgsl");
        embedded_asset!(app, "ssao.wgsl");
        embedded_asset!(app, "spatial_denoise.wgsl");
    }

    fn plugin_render_app_build(render_app: &mut SubApp) {
        render_app.init_resource::<SsaoStaticResources>();
    }

    fn encode_commands(
        &self,
        mut ctx: RenderTaskContext,
        camera_entity: Entity,
        world: &World,
    ) -> Option<()> {
        let (camera, prepass_textures, view_uniform_offset, has_temporal_jitter) =
            world.entity(camera_entity).get_components::<(
                &ExtractedCamera,
                &ViewPrepassTextures,
                &ViewUniformOffset,
                Has<TemporalJitter>,
            )>()?;
        let render_adapter = world.get_resource::<RenderAdapter>()?;
        let view_uniforms = world.get_resource::<ViewUniforms>()?.uniforms.buffer()?;
        let global_uniforms = world.get_resource::<GlobalsBuffer>()?.buffer.buffer()?;
        let static_resources = world.get_resource::<SsaoStaticResources>()?;
        let view_uniform_offset = view_uniform_offset.offset;

        let camera_size = camera.physical_viewport_size?.to_extents();
        let depth_format = get_depth_format(render_adapter);
        let (slice_count, samples_per_slice_side) = self.quality_level.sample_counts();

        // TODO: Helpers/builder pattern for texture descriptor creation
        let preprocessed_depth_texture = ctx.texture(TextureDescriptor {
            label: Some("ssao_preprocessed_depth_texture"),
            size: camera_size,
            mip_level_count: 5,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: depth_format,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let preprocessed_depth_texture_view = |mip_level| -> TextureView {
            let texture_view_descriptor = TextureViewDescriptor {
                label: Some("ssao_preprocessed_depth_texture_mip_view"),
                base_mip_level: mip_level,
                format: Some(depth_format),
                dimension: Some(TextureViewDimension::D2),
                mip_level_count: Some(1),
                ..default()
            };

            preprocessed_depth_texture
                .texture()
                .create_view(&texture_view_descriptor)
                .into()
        };

        let ssao_noisy_texture = ctx.texture(TextureDescriptor {
            label: Some("ssao_noisy_texture"),
            size: camera_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: depth_format,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // TODO: How does prepare_mesh_view_bind_groups() get access to this texture?
        // Might need to create this one specifically outside of RenderTask
        let ssao_texture = ctx.texture(TextureDescriptor {
            label: Some("ssao_texture"),
            size: camera_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: depth_format,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_differences_texture = ctx.texture(TextureDescriptor {
            label: Some("ssao_depth_differences_texture"),
            size: camera_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Uint,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let render_device = world.get_resource::<RenderDevice>()?;
        let thickness_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("thickness_buffer"),
            contents: &self.constant_object_thickness.to_le_bytes(),
            usage: BufferUsages::UNIFORM,
        });

        let common_resources = (
            (
                SamplerNonFiltering(&static_resources.point_clamp_sampler),
                SamplerFiltering(&static_resources.linear_clamp_sampler),
                DynamicUniformBuffer(view_uniforms),
            ),
            [view_uniform_offset].as_slice(),
        );

        ctx.compute_pass("preprocess_depth")
            .shader(load_embedded_asset!(world, "preprocess_depth.wgsl"))
            .shader_def_if("USE_R16FLOAT", depth_format == TextureFormat::R16Float)
            .bind_resources((
                SampledTexture(prepass_textures.depth_view()?),
                StorageTextureWriteOnly(&preprocessed_depth_texture_view(0)),
                StorageTextureWriteOnly(&preprocessed_depth_texture_view(1)),
                StorageTextureWriteOnly(&preprocessed_depth_texture_view(2)),
                StorageTextureWriteOnly(&preprocessed_depth_texture_view(3)),
                StorageTextureWriteOnly(&preprocessed_depth_texture_view(4)),
            ))
            .bind_resources_with_dynamic_offsets(common_resources)
            .dispatch_2d(
                camera_size.width.div_ceil(16),
                camera_size.height.div_ceil(16),
            )?;

        ctx.compute_pass("ssao")
            .shader(load_embedded_asset!(world, "ssao.wgsl"))
            .shader_def_if("USE_R16FLOAT", depth_format == TextureFormat::R16Float)
            .shader_def_if("TEMPORAL_JITTER", has_temporal_jitter)
            .shader_def(ShaderDefVal::Int("SLICE_COUNT".to_owned(), slice_count))
            .shader_def(ShaderDefVal::Int(
                // TODO: Better API for making ShaderDefVals
                "SAMPLES_PER_SLICE_SIDE".to_owned(),
                samples_per_slice_side,
            ))
            .bind_resources((
                SampledTexture(&preprocessed_depth_texture),
                SampledTexture(prepass_textures.normal_view()?),
                SampledTexture(&static_resources.hilbert_index_lut),
                StorageTextureWriteOnly(&ssao_noisy_texture),
                StorageTextureWriteOnly(&depth_differences_texture),
                UniformBuffer(global_uniforms),
                UniformBuffer(&thickness_buffer),
            ))
            .bind_resources_with_dynamic_offsets(common_resources)
            .dispatch_2d(
                camera_size.width.div_ceil(8),
                camera_size.height.div_ceil(8),
            )?;

        ctx.compute_pass("ssao_spatial_denoise")
            .shader(load_embedded_asset!(world, "spatial_denoise.wgsl"))
            .shader_def_if("USE_R16FLOAT", depth_format == TextureFormat::R16Float)
            .bind_resources((
                SampledTexture(&ssao_noisy_texture),
                SampledTexture(&depth_differences_texture),
                StorageTextureWriteOnly(&ssao_texture),
            ))
            .bind_resources_with_dynamic_offsets(common_resources)
            .dispatch_2d(
                camera_size.width.div_ceil(8), // TODO: Helpers for this kind of dispatch?
                camera_size.height.div_ceil(8),
            )?;

        Some(())
    }
}

#[derive(Resource)]
struct SsaoStaticResources {
    hilbert_index_lut: TextureView,
    point_clamp_sampler: Sampler,
    linear_clamp_sampler: Sampler,
}

impl FromWorld for SsaoStaticResources {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let texture_descriptor = TextureDescriptor {
            label: Some("ssao_hilbert_index_lut"),
            size: Extent3d {
                width: HILBERT_WIDTH as u32,
                height: HILBERT_WIDTH as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let hilbert_index_lut = render_device
            .create_texture_with_data(
                render_queue,
                &texture_descriptor,
                TextureDataOrder::default(),
                bytemuck::cast_slice(&generate_hilbert_index_lut()),
            )
            .create_view(&TextureViewDescriptor::default());

        let point_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let linear_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            hilbert_index_lut,
            point_clamp_sampler,
            linear_clamp_sampler,
        }
    }
}

fn generate_hilbert_index_lut() -> [[u16; 64]; 64] {
    use core::array::from_fn;
    from_fn(|x| from_fn(|y| hilbert_index(x as u16, y as u16)))
}

// https://www.shadertoy.com/view/3tB3z3
const HILBERT_WIDTH: u16 = 64;
fn hilbert_index(mut x: u16, mut y: u16) -> u16 {
    let mut index = 0;

    let mut level: u16 = HILBERT_WIDTH / 2;
    while level > 0 {
        let region_x = (x & level > 0) as u16;
        let region_y = (y & level > 0) as u16;
        index += level * level * ((3 * region_x) ^ region_y);

        if region_y == 0 {
            if region_x == 1 {
                x = HILBERT_WIDTH - 1 - x;
                y = HILBERT_WIDTH - 1 - y;
            }

            mem::swap(&mut x, &mut y);
        }

        level /= 2;
    }

    index
}

fn get_depth_format(render_adapter: &RenderAdapter) -> TextureFormat {
    if render_adapter
        .get_texture_format_features(TextureFormat::R16Float)
        .allowed_usages
        .contains(TextureUsages::STORAGE_BINDING)
    {
        TextureFormat::R16Float
    } else {
        TextureFormat::R32Float
    }
}
