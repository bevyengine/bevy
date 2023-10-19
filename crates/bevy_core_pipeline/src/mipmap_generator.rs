//! A wrapper that runs a shader repeatedly to populate a series of mipmaps.
//!
//! A variety of passes need to or will need to do this, and using this module
//! helps to reduce boilerplate and code duplication.

use std::{borrow::Cow, hash::Hash, marker::PhantomData};

use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    system::Resource,
    world::{FromWorld, World},
};
use bevy_math::UVec2;
use bevy_render::{
    render_resource::{
        AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState,
        LoadOp, MultisampleState, Operations, PipelineCache, PrimitiveState,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
        SamplerBindingType, SamplerDescriptor, Shader, ShaderDefVal, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
        TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, TextureCache},
    RenderApp,
};

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;

/// All mipmappers implement this trait.
///
/// The type that implements this trait is used as the pipeline key for the mipmapping shader.
pub trait Mipmap: Clone + PartialEq + Eq + Hash + Send + Sync + 'static {
    /// Returns a handle to the mipmapping shader.
    ///
    /// This shader is expected to have a 2D input texture at group 0, binding 0
    /// and a sampler to sample that texture with at group 0, binding 1. It'll
    /// be invoked repeatedly in order to generate smaller and smaller mip
    /// levels.
    fn shader() -> Handle<Shader>;

    /// Returns the desired format for the mipmap texture.
    fn texture_format() -> TextureFormat;

    /// Returns the name of the entry point for the shader.
    fn shader_entry_point(first: bool) -> Cow<'static, str>;

    /// Returns a structure with various labels for `wgpu` objects.
    fn debug_names() -> &'static MipmapDebugNames;

    /// Adds bind group layout entries for any custom inputs to the shader.
    fn add_custom_bind_group_layout_entries(_entries: &mut Vec<BindGroupLayoutEntry>) {}

    /// Given the pipeline key, adds any needed custom shader definitions to the shader.
    fn add_custom_shader_defs(&self, _shader_defs: &mut Vec<ShaderDefVal>) {}

    /// The number of mip levels to omit, starting from the smallest.
    ///
    /// For example, if this value is 1, the smallest mip level is omitted.
    fn mip_levels_to_omit() -> u32 {
        0
    }
}

/// A plugin that adds various resources associated with the mipmapper.
pub struct MipmapPlugin<M>
where
    M: Mipmap,
{
    phantom: PhantomData<M>,
}

/// Labels for various `wgpu` objects.
///
/// These show up in debugging tools like `RenderDoc`.
pub struct MipmapDebugNames {
    /// The label for the mipmapper's bind group layout.
    pub bind_group_layout: &'static str,
    /// The label for the mipmap texture.
    pub texture: &'static str,
    /// The label for the pipeline associated with the shader invocation that
    /// renders to mip level 0.
    pub first_pipeline: &'static str,
    /// The label for the pipeline associated with the shader invocation that
    /// renders to mip levels greater than 0.
    pub rest_pipeline: &'static str,
    /// The label for the bind group associated with the shader invocation that
    /// renders to mip level 0.
    pub first_bind_group: &'static str,
    /// The label for the bind group associated with the shader invocation that
    /// renders to mip levels greater than 0.
    pub rest_bind_group: &'static str,
    /// The label for the render pass associated with the shader invocation that
    /// renders to mip level 0.
    pub first_pass: &'static str,
    /// The label for the render pass associated with the shader invocation that
    /// renders to mip levels greater than 0.
    pub rest_pass: &'static str,
}

/// IDs for the render pipelines associated with the mipmapper.
///
/// This implements Component as a convenience, so that you can attach it to
/// entities if you wish.
#[derive(Component)]
pub struct MipmapPipelineIds<T>
where
    T: Mipmap,
{
    /// The ID of the pipeline associated with the shader invocation that
    /// renders to mip level 0.
    pub first: CachedRenderPipelineId,
    /// The ID of the pipeline associated with the shader invocation that
    /// renders to mip levels greater than 0.
    pub rest: CachedRenderPipelineId,
    phantom: PhantomData<T>,
}

/// The bind group layout and sampler for the render pipelines associated with
/// the mipmapper.
///
/// This is a singleton resource.
#[derive(Resource)]
pub struct MipmapPipeline<T>
where
    T: Mipmap,
{
    /// The bind group associated with the shader.
    ///
    /// Group 0, binding 0 is the source image. Group 0, binding 1 is the
    /// sampler.
    pub bind_group_layout: BindGroupLayout,

    /// The sampler that's used to sample from binding 0.
    ///
    /// This is assigned to group 0, binding 1.
    pub sampler: Sampler,

    phantom: PhantomData<T>,
}

/// The key that identifies mipmapper render pipelines.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MipmapPipelineKey<M>
where
    M: Mipmap,
{
    /// True if this is the shader invocation that generates mip level 0.
    pub first: bool,

    /// The rest of the pipeline key.
    ///
    /// You can store custom data in here if you need to customize the shader in
    /// some way.
    pub mipmapper: M,
}

/// The texture or textures that store the mip levels generated by this
/// mipmapper.
#[derive(Component)]
pub struct MipmappedTexture<M>
where
    M: Mipmap,
{
    /// The texture that stores the mipmap levels.
    ///
    /// On WebGL 2, it's not possible to create texture views of individual
    /// mipmap levels. So we store separate textures for each mip level on that
    /// platform.
    #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
    pub cached_texture: CachedTexture,

    /// The texture that stores the mipmap levels.
    ///
    /// On WebGL 2, it's not possible to create texture views of individual
    /// mipmap levels. So we store separate textures for each mip level on that
    /// platform.
    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    pub texture: Vec<CachedTexture>,

    /// The number of mip levels.
    pub mip_count: u32,

    phantom: PhantomData<M>,
}

/// Bind groups associated with the mipmapper's shader invocations.
///
/// As a convenience, this derives Component so that you can attach it to
/// entities if you wish.
#[derive(Component)]
pub struct MipmapBindGroups<M>
where
    M: Mipmap,
{
    bind_groups: Box<[BindGroup]>,
    sampler: Sampler,
    phantom: PhantomData<M>,
}

impl<M> Plugin for MipmapPlugin<M>
where
    M: Mipmap,
{
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<SpecializedRenderPipelines<MipmapPipeline<M>>>();
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<MipmapPipeline<M>>();
    }
}

impl<M> FromWorld for MipmapPipeline<M>
where
    M: Mipmap,
{
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create the common bind group layout entries.
        let mut bind_group_layout_entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                visibility: ShaderStages::FRAGMENT,
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                visibility: ShaderStages::FRAGMENT,
                count: None,
            },
        ];

        // Ask the mipmapper to add any custom entries.
        M::add_custom_bind_group_layout_entries(&mut bind_group_layout_entries);

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(M::debug_names().bind_group_layout),
                entries: &bind_group_layout_entries,
            });

        Self {
            bind_group_layout,
            sampler: render_device.create_sampler(&SamplerDescriptor {
                min_filter: FilterMode::Linear,
                mag_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                ..SamplerDescriptor::default()
            }),
            phantom: PhantomData,
        }
    }
}

impl<M> SpecializedRenderPipeline for MipmapPipeline<M>
where
    M: Mipmap,
{
    type Key = MipmapPipelineKey<M>;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Define `FIRST_DOWNSAMPLE` if this is the invocation that renders to
        // mipmap level 0.
        let mut shader_defs = vec![];
        if key.first {
            shader_defs.push("FIRST_DOWNSAMPLE".into());
        }

        // Add any custom shader definitions as appropriate.
        key.mipmapper.add_custom_shader_defs(&mut shader_defs);

        // Build the descriptor.
        RenderPipelineDescriptor {
            label: Some(if key.first {
                M::debug_names().first_pipeline.into()
            } else {
                M::debug_names().rest_pipeline.into()
            }),
            layout: vec![self.bind_group_layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: M::shader(),
                shader_defs,
                entry_point: M::shader_entry_point(key.first),
                targets: vec![Some(ColorTargetState {
                    format: M::texture_format(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        }
    }
}

impl<M> MipmapPipelineIds<M>
where
    M: Mipmap,
{
    /// Creates pipelines for the mipmapping shader invocations.
    pub fn new(
        mipmapper: M,
        pipeline_cache: &PipelineCache,
        pipelines: &mut SpecializedRenderPipelines<MipmapPipeline<M>>,
        pipeline: &MipmapPipeline<M>,
    ) -> Self {
        let first = pipelines.specialize(
            pipeline_cache,
            pipeline,
            MipmapPipelineKey {
                mipmapper: mipmapper.clone(),
                first: true,
            },
        );

        let rest = pipelines.specialize(
            pipeline_cache,
            pipeline,
            MipmapPipelineKey {
                mipmapper,
                first: false,
            },
        );

        MipmapPipelineIds {
            first,
            rest,
            phantom: PhantomData,
        }
    }
}

impl<M> MipmappedTexture<M>
where
    M: Mipmap,
{
    /// Creates a new mipmapped texture.
    ///
    /// The given `size` is the size of mipmap level 0. It need not be square.
    /// The number of mip levels is automatically calculated based on that
    /// value.
    pub fn new(
        render_device: &RenderDevice,
        texture_cache: &mut TextureCache,
        size: UVec2,
    ) -> Self {
        let mip_count = size.min_element().ilog2().max(2) - M::mip_levels_to_omit();
        let aspect_ratio = size.x as f32 / size.y as f32;

        let texture_descriptor = TextureDescriptor {
            label: Some(M::debug_names().texture),
            size: if aspect_ratio >= 1.0 {
                Extent3d {
                    width: ((size.x as f32).round() as u32).max(1),
                    height: ((size.x as f32 / aspect_ratio).round() as u32).max(1),
                    depth_or_array_layers: 1,
                }
            } else {
                Extent3d {
                    width: ((size.y as f32 * aspect_ratio).round() as u32).max(1),
                    height: ((size.y as f32).round() as u32).max(1),
                    depth_or_array_layers: 1,
                }
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: M::texture_format(),
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
        let cached_texture = texture_cache.get(render_device, texture_descriptor);

        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        let cached_texture: Vec<CachedTexture> = (0..mip_count)
            .map(|mip_level| {
                texture_cache.get(
                    &render_device,
                    TextureDescriptor {
                        size: Extent3d {
                            width: (texture_descriptor.size.width >> mip).max(1),
                            height: (texture_descriptor.size.height >> mip).max(1),
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        ..texture_descriptor.clone()
                    },
                )
            })
            .collect();

        MipmappedTexture {
            cached_texture,
            mip_count,
            phantom: PhantomData,
        }
    }

    /// Returns a 2D texture view of the given mip level.
    #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
    pub fn view(&self, mip_level: u32) -> TextureView {
        self.cached_texture
            .texture
            .create_view(&TextureViewDescriptor {
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                ..TextureViewDescriptor::default()
            })
    }

    /// Returns a 2D texture view of the given mip level.
    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    pub fn view(&self, mip_level: u32) -> TextureView {
        self.cached_texture[mip_level as usize]
            .texture
            .create_view(&TextureViewDescriptor {
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                ..TextureViewDescriptor::default()
            })
    }
}

impl<M> MipmapBindGroups<M>
where
    M: Mipmap,
{
    /// Populates the bind groups associated with the mipmapping shader
    /// invocation.
    pub fn new(
        render_device: &RenderDevice,
        pipeline: &MipmapPipeline<M>,
        texture: &MipmappedTexture<M>,
        custom_bind_group_entries: &[BindGroupEntry],
    ) -> Self {
        let mut bind_groups = Vec::with_capacity(texture.mip_count as usize - 1);
        for src_mip_level in 0..(texture.mip_count - 1) {
            let texture_view = texture.view(src_mip_level);
            let mut bind_group_entries = vec![
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
            ];

            bind_group_entries.extend(custom_bind_group_entries.iter().cloned());

            bind_groups.push(render_device.create_bind_group(&BindGroupDescriptor {
                label: Some(M::debug_names().rest_bind_group),
                layout: &pipeline.bind_group_layout,
                entries: &bind_group_entries,
            }));
        }

        Self {
            bind_groups: bind_groups.into_boxed_slice(),
            sampler: pipeline.sampler.clone(),
            phantom: PhantomData,
        }
    }
}

/// Adds render commands to run the shader repeatedly, populating each mip
/// level.
///
/// `source` is the input texture for the first mip level. After that, mip level
/// N - 1 is used as the input texture for generation of mip level N.
///
/// `custom_bind_group_entries` and `dynamic_uniform_indices` are useful if you
/// need to pass extra data into your mipmap generation shader. You can set them
/// to empty arrays if you don't need any extra inputs.
#[allow(clippy::too_many_arguments)]
pub fn generate_mipmaps<M>(
    render_context: &mut RenderContext,
    pipeline_cache: &PipelineCache,
    pipeline: &MipmapPipeline<M>,
    pipeline_ids: &MipmapPipelineIds<M>,
    bind_groups: &MipmapBindGroups<M>,
    texture: &MipmappedTexture<M>,
    source: &TextureView,
    custom_bind_group_entries: &[BindGroupEntry],
    dynamic_uniform_indices: &[u32],
) -> bool
where
    M: Mipmap,
{
    let Some(first_pipeline) = pipeline_cache.get_render_pipeline(pipeline_ids.first) else {
        return false;
    };
    let Some(rest_pipeline) = pipeline_cache.get_render_pipeline(pipeline_ids.rest) else {
        return false;
    };

    // Run the first pass.

    {
        let mut bind_group_entries = vec![
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(source),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&bind_groups.sampler),
            },
        ];

        bind_group_entries.extend(custom_bind_group_entries.iter().cloned());

        let first_bind_group =
            render_context
                .render_device()
                .create_bind_group(&BindGroupDescriptor {
                    label: Some(M::debug_names().first_bind_group),
                    layout: &pipeline.bind_group_layout,
                    entries: &bind_group_entries,
                });

        let view = &texture.view(0);

        let mut first_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some(M::debug_names().first_pass),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        });

        first_pass.set_render_pipeline(first_pipeline);
        first_pass.set_bind_group(0, &first_bind_group, dynamic_uniform_indices);
        first_pass.draw(0..3, 0..1);
    }

    // Rest of the passes

    for dest_mip_level in 1..(texture.mip_count - 1) {
        let view = &texture.view(dest_mip_level);
        let mut rest_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some(M::debug_names().rest_pass),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        rest_pass.set_render_pipeline(rest_pipeline);
        rest_pass.set_bind_group(
            0,
            &bind_groups.bind_groups[dest_mip_level as usize - 1],
            dynamic_uniform_indices,
        );
        rest_pass.draw(0..3, 0..1);
    }

    true
}

impl<M> Default for MipmapPlugin<M>
where
    M: Mipmap,
{
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}
