use core::ops::Range;

use crate::ComputedTextureSlices;
use bevy_asset::{load_embedded_asset, AssetEvent, AssetId, AssetServer, Assets, Handle};
use bevy_camera::visibility::ViewVisibility;
use bevy_color::{ColorToComponents, LinearRgba};
use bevy_core_pipeline::{
    core_2d::{Transparent2d, CORE_2D_DEPTH_FORMAT},
    tonemapping::{
        get_lut_bind_group_layout_entries, get_lut_bindings, DebandDither, Tonemapping,
        TonemappingLuts,
    },
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::*,
    query::ROQueryItem,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_image::{BevyDefault, Image, ImageSampler, TextureAtlasLayout, TextureFormatPixelInfo};
use bevy_math::{Affine3A, FloatOrd, Quat, Rect, Vec2, Vec4};
use bevy_mesh::VertexBufferLayout;
use bevy_platform::collections::HashMap;
use bevy_render::view::{RenderVisibleEntities, RetainedViewEntity};
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::{
        DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
        SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::{RenderDevice, RenderQueue},
    sync_world::RenderEntity,
    texture::{DefaultImageSampler, FallbackImage, GpuImage},
    view::{ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_sprite::{Anchor, ScalingMode, Sprite};
use bevy_transform::components::GlobalTransform;
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

#[derive(Resource)]
pub struct SpritePipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
    shader: Handle<Shader>,
    pub dummy_white_gpu_image: GpuImage,
}

pub fn init_sprite_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    default_sampler: Res<DefaultImageSampler>,
    render_queue: Res<RenderQueue>,
    asset_server: Res<AssetServer>,
) {
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
    let view_layout = render_device.create_bind_group_layout(
        "sprite_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                tonemapping_lut_entries[0].visibility(ShaderStages::FRAGMENT),
                tonemapping_lut_entries[1].visibility(ShaderStages::FRAGMENT),
            ),
        ),
    );

    let material_layout = render_device.create_bind_group_layout(
        "sprite_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    let dummy_white_gpu_image = {
        let image = Image::default();
        let texture = render_device.create_texture(&image.texture_descriptor);
        let sampler = match image.sampler {
            ImageSampler::Default => (**default_sampler).clone(),
            ImageSampler::Descriptor(ref descriptor) => {
                render_device.create_sampler(&descriptor.as_wgpu())
            }
        };

        if let Ok(format_size) = image.texture_descriptor.format.pixel_size() {
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
        }
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

    commands.insert_resource(SpritePipeline {
        view_layout,
        material_layout,
        dummy_white_gpu_image,
        shader: load_embedded_asset!(asset_server.as_ref(), "sprite.wgsl"),
    });
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct SpritePipelineKey: u32 {
        const NONE                              = 0;
        const HDR                               = 1 << 0;
        const TONEMAP_IN_SHADER                 = 1 << 1;
        const DEBAND_DITHER                     = 1 << 2;
        const MSAA_RESERVED_BITS                = Self::MSAA_MASK_BITS << Self::MSAA_SHIFT_BITS;
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

impl SpritePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();
    const TONEMAP_METHOD_MASK_BITS: u32 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u32 =
        Self::MSAA_SHIFT_BITS - Self::TONEMAP_METHOD_MASK_BITS.count_ones();

    #[inline]
    pub const fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits =
            (msaa_samples.trailing_zeros() & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        Self::from_bits_retain(msaa_bits)
    }

    #[inline]
    pub const fn msaa_samples(&self) -> u32 {
        1 << ((self.bits() >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS)
    }

    #[inline]
    pub const fn from_hdr(hdr: bool) -> Self {
        if hdr {
            SpritePipelineKey::HDR
        } else {
            SpritePipelineKey::NONE
        }
    }
}

impl SpecializedRenderPipeline for SpritePipeline {
    type Key = SpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.contains(SpritePipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                1,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                2,
            ));

            let method = key.intersection(SpritePipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == SpritePipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
            {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(SpritePipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        let format = match key.contains(SpritePipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        let instance_rate_vertex_buffer_layout = VertexBufferLayout {
            array_stride: 80,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // @location(0) i_model_transpose_col0: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                // @location(1) i_model_transpose_col1: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 1,
                },
                // @location(2) i_model_transpose_col2: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 2,
                },
                // @location(3) i_color: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 3,
                },
                // @location(4) i_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 64,
                    shader_location: 4,
                },
            ],
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            layout: vec![self.view_layout.clone(), self.material_layout.clone()],
            // Sprites are always alpha blended so they never need to write to depth.
            // They just need to read it in case an opaque mesh2d
            // that wrote to depth is present.
            depth_stencil: Some(DepthStencilState {
                format: CORE_2D_DEPTH_FORMAT,
                depth_write_enabled: false,
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
            label: Some("sprite_pipeline".into()),
            ..default()
        }
    }
}

pub struct ExtractedSlice {
    pub offset: Vec2,
    pub rect: Rect,
    pub size: Vec2,
}

pub struct ExtractedSprite {
    pub main_entity: Entity,
    pub render_entity: Entity,
    pub transform: GlobalTransform,
    pub color: LinearRgba,
    /// Change the on-screen size of the sprite
    /// Asset ID of the [`Image`] of this sprite
    /// PERF: storing an `AssetId` instead of `Handle<Image>` enables some optimizations (`ExtractedSprite` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: AssetId<Image>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub kind: ExtractedSpriteKind,
}

pub enum ExtractedSpriteKind {
    /// A single sprite with custom sizing and scaling options
    Single {
        anchor: Vec2,
        rect: Option<Rect>,
        scaling_mode: Option<ScalingMode>,
        custom_size: Option<Vec2>,
    },
    /// Indexes into the list of [`ExtractedSlice`]s stored in the [`ExtractedSlices`] resource
    /// Used for elements composed from multiple sprites such as text or nine-patched borders
    Slices { indices: Range<usize> },
}

#[derive(Resource, Default)]
pub struct ExtractedSprites {
    pub sprites: Vec<ExtractedSprite>,
}

#[derive(Resource, Default)]
pub struct ExtractedSlices {
    pub slices: Vec<ExtractedSlice>,
}

#[derive(Resource, Default)]
pub struct SpriteAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub fn extract_sprite_events(
    mut events: ResMut<SpriteAssetEvents>,
    mut image_events: Extract<MessageReader<AssetEvent<Image>>>,
) {
    let SpriteAssetEvents { ref mut images } = *events;
    images.clear();

    for event in image_events.read() {
        images.push(*event);
    }
}

pub fn extract_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    mut extracted_slices: ResMut<ExtractedSlices>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    sprite_query: Extract<
        Query<(
            Entity,
            RenderEntity,
            &ViewVisibility,
            &Sprite,
            &GlobalTransform,
            &Anchor,
            Option<&ComputedTextureSlices>,
        )>,
    >,
) {
    extracted_sprites.sprites.clear();
    extracted_slices.slices.clear();
    for (main_entity, render_entity, view_visibility, sprite, transform, anchor, slices) in
        sprite_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }

        if let Some(slices) = slices {
            let start = extracted_slices.slices.len();
            extracted_slices
                .slices
                .extend(slices.extract_slices(sprite, anchor.as_vec()));
            let end = extracted_slices.slices.len();
            extracted_sprites.sprites.push(ExtractedSprite {
                main_entity,
                render_entity,
                color: sprite.color.into(),
                transform: *transform,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: sprite.image.id(),
                kind: ExtractedSpriteKind::Slices {
                    indices: start..end,
                },
            });
        } else {
            let atlas_rect = sprite
                .texture_atlas
                .as_ref()
                .and_then(|s| s.texture_rect(&texture_atlases).map(|r| r.as_rect()));
            let rect = match (atlas_rect, sprite.rect) {
                (None, None) => None,
                (None, Some(sprite_rect)) => Some(sprite_rect),
                (Some(atlas_rect), None) => Some(atlas_rect),
                (Some(atlas_rect), Some(mut sprite_rect)) => {
                    sprite_rect.min += atlas_rect.min;
                    sprite_rect.max += atlas_rect.min;
                    Some(sprite_rect)
                }
            };

            // PERF: we don't check in this function that the `Image` asset is ready, since it should be in most cases and hashing the handle is expensive
            extracted_sprites.sprites.push(ExtractedSprite {
                main_entity,
                render_entity,
                color: sprite.color.into(),
                transform: *transform,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: sprite.image.id(),
                kind: ExtractedSpriteKind::Single {
                    anchor: anchor.as_vec(),
                    rect,
                    scaling_mode: sprite.image_mode.scale(),
                    // Pass the custom size
                    custom_size: sprite.custom_size,
                },
            });
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpriteInstance {
    // Affine 4x3 transposed to 3x4
    pub i_model_transpose: [Vec4; 3],
    pub i_color: [f32; 4],
    pub i_uv_offset_scale: [f32; 4],
}

impl SpriteInstance {
    #[inline]
    fn from(transform: &Affine3A, color: &LinearRgba, uv_offset_scale: &Vec4) -> Self {
        let transpose_model_3x3 = transform.matrix3.transpose();
        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(transform.translation.x),
                transpose_model_3x3.y_axis.extend(transform.translation.y),
                transpose_model_3x3.z_axis.extend(transform.translation.z),
            ],
            i_color: color.to_f32_array(),
            i_uv_offset_scale: uv_offset_scale.to_array(),
        }
    }
}

#[derive(Resource)]
pub struct SpriteMeta {
    sprite_index_buffer: RawBufferVec<u32>,
    sprite_instance_buffer: RawBufferVec<SpriteInstance>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            sprite_index_buffer: RawBufferVec::<u32>::new(BufferUsages::INDEX),
            sprite_instance_buffer: RawBufferVec::<SpriteInstance>::new(BufferUsages::VERTEX),
        }
    }
}

#[derive(Component)]
pub struct SpriteViewBindGroup {
    pub value: BindGroup,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct SpriteBatches(HashMap<(RetainedViewEntity, Entity), SpriteBatch>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SpriteBatch {
    image_handle_id: AssetId<Image>,
    range: Range<u32>,
}

#[derive(Resource, Default)]
pub struct ImageBindGroups {
    values: HashMap<AssetId<Image>, BindGroup>,
}

pub fn queue_sprites(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    sprite_pipeline: Res<SpritePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SpritePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    extracted_sprites: Res<ExtractedSprites>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut views: Query<(
        &RenderVisibleEntities,
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
) {
    let draw_sprite_function = draw_functions.read().id::<DrawSprite>();

    for (visible_entities, view, msaa, tonemapping, dither) in &mut views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let msaa_key = SpritePipelineKey::from_msaa_samples(msaa.samples());
        let mut view_key = SpritePipelineKey::from_hdr(view.hdr) | msaa_key;

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= SpritePipelineKey::TONEMAP_IN_SHADER;
                view_key |= match tonemapping {
                    Tonemapping::None => SpritePipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => SpritePipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        SpritePipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => SpritePipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => SpritePipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        SpritePipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => SpritePipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => SpritePipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= SpritePipelineKey::DEBAND_DITHER;
            }
        }

        let pipeline = pipelines.specialize(&pipeline_cache, &sprite_pipeline, view_key);

        view_entities.clear();
        view_entities.extend(
            visible_entities
                .iter::<Sprite>()
                .map(|(_, e)| e.index() as usize),
        );

        transparent_phase
            .items
            .reserve(extracted_sprites.sprites.len());

        for (index, extracted_sprite) in extracted_sprites.sprites.iter().enumerate() {
            let view_index = extracted_sprite.main_entity.index();

            if !view_entities.contains(view_index as usize) {
                continue;
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_sprite.transform.translation().z);

            // Add the item to the render phase
            transparent_phase.add(Transparent2d {
                draw_function: draw_sprite_function,
                pipeline,
                entity: (
                    extracted_sprite.render_entity,
                    extracted_sprite.main_entity.into(),
                ),
                sort_key,
                // `batch_range` is calculated in `prepare_sprite_image_bind_groups`
                batch_range: 0..0,
                extra_index: PhaseItemExtraIndex::None,
                extracted_index: index,
                indexed: true,
            });
        }
    }
}

pub fn prepare_sprite_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    sprite_pipeline: Res<SpritePipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &Tonemapping), With<ExtractedView>>,
    tonemapping_luts: Res<TonemappingLuts>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
) {
    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        return;
    };

    for (entity, tonemapping) in &views {
        let lut_bindings =
            get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
        let view_bind_group = render_device.create_bind_group(
            "mesh2d_view_bind_group",
            &sprite_pipeline.view_layout,
            &BindGroupEntries::sequential((view_binding.clone(), lut_bindings.0, lut_bindings.1)),
        );

        commands.entity(entity).insert(SpriteViewBindGroup {
            value: view_bind_group,
        });
    }
}

pub fn prepare_sprite_image_bind_groups(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta>,
    sprite_pipeline: Res<SpritePipeline>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    extracted_sprites: Res<ExtractedSprites>,
    extracted_slices: Res<ExtractedSlices>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    events: Res<SpriteAssetEvents>,
    mut batches: ResMut<SpriteBatches>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Unused { id } | AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
            }
        };
    }

    batches.clear();

    // Clear the sprite instances
    sprite_meta.sprite_instance_buffer.clear();

    // Index buffer indices
    let mut index = 0;

    let image_bind_groups = &mut *image_bind_groups;

    for (retained_view, transparent_phase) in phases.iter_mut() {
        let mut current_batch = None;
        let mut batch_item_index = 0;
        let mut batch_image_size = Vec2::ZERO;
        let mut batch_image_handle = AssetId::invalid();

        // Iterate through the phase items and detect when successive sprites that can be batched.
        // Spawn an entity with a `SpriteBatch` component for each possible batch.
        // Compatible items share the same entity.
        for item_index in 0..transparent_phase.items.len() {
            let item = &transparent_phase.items[item_index];

            let Some(extracted_sprite) = extracted_sprites
                .sprites
                .get(item.extracted_index)
                .filter(|extracted_sprite| extracted_sprite.render_entity == item.entity())
            else {
                // If there is a phase item that is not a sprite, then we must start a new
                // batch to draw the other phase item(s) and to respect draw order. This can be
                // done by invalidating the batch_image_handle
                batch_image_handle = AssetId::invalid();
                continue;
            };

            if batch_image_handle != extracted_sprite.image_handle_id {
                let Some(gpu_image) = gpu_images.get(extracted_sprite.image_handle_id) else {
                    continue;
                };

                batch_image_size = gpu_image.size_2d().as_vec2();
                batch_image_handle = extracted_sprite.image_handle_id;
                image_bind_groups
                    .values
                    .entry(batch_image_handle)
                    .or_insert_with(|| {
                        render_device.create_bind_group(
                            "sprite_material_bind_group",
                            &sprite_pipeline.material_layout,
                            &BindGroupEntries::sequential((
                                &gpu_image.texture_view,
                                &gpu_image.sampler,
                            )),
                        )
                    });

                batch_item_index = item_index;
                current_batch = Some(batches.entry((*retained_view, item.entity())).insert(
                    SpriteBatch {
                        image_handle_id: batch_image_handle,
                        range: index..index,
                    },
                ));
            }
            match extracted_sprite.kind {
                ExtractedSpriteKind::Single {
                    anchor,
                    rect,
                    scaling_mode,
                    custom_size,
                } => {
                    // By default, the size of the quad is the size of the texture
                    let mut quad_size = batch_image_size;
                    let mut texture_size = batch_image_size;

                    // Calculate vertex data for this item
                    // If a rect is specified, adjust UVs and the size of the quad
                    let mut uv_offset_scale = if let Some(rect) = rect {
                        let rect_size = rect.size();
                        quad_size = rect_size;
                        // Update texture size to the rect size
                        // It will help scale properly only portion of the image
                        texture_size = rect_size;
                        Vec4::new(
                            rect.min.x / batch_image_size.x,
                            rect.max.y / batch_image_size.y,
                            rect_size.x / batch_image_size.x,
                            -rect_size.y / batch_image_size.y,
                        )
                    } else {
                        Vec4::new(0.0, 1.0, 1.0, -1.0)
                    };

                    if extracted_sprite.flip_x {
                        uv_offset_scale.x += uv_offset_scale.z;
                        uv_offset_scale.z *= -1.0;
                    }
                    if extracted_sprite.flip_y {
                        uv_offset_scale.y += uv_offset_scale.w;
                        uv_offset_scale.w *= -1.0;
                    }

                    // Override the size if a custom one is specified
                    quad_size = custom_size.unwrap_or(quad_size);

                    // Used for translation of the quad if `TextureScale::Fit...` is specified.
                    let mut quad_translation = Vec2::ZERO;

                    // Scales the texture based on the `texture_scale` field.
                    if let Some(scaling_mode) = scaling_mode {
                        apply_scaling(
                            scaling_mode,
                            texture_size,
                            &mut quad_size,
                            &mut quad_translation,
                            &mut uv_offset_scale,
                        );
                    }

                    let transform = extracted_sprite.transform.affine()
                        * Affine3A::from_scale_rotation_translation(
                            quad_size.extend(1.0),
                            Quat::IDENTITY,
                            ((quad_size + quad_translation) * (-anchor - Vec2::splat(0.5)))
                                .extend(0.0),
                        );

                    // Store the vertex data and add the item to the render phase
                    sprite_meta
                        .sprite_instance_buffer
                        .push(SpriteInstance::from(
                            &transform,
                            &extracted_sprite.color,
                            &uv_offset_scale,
                        ));

                    current_batch.as_mut().unwrap().get_mut().range.end += 1;
                    index += 1;
                }
                ExtractedSpriteKind::Slices { ref indices } => {
                    for i in indices.clone() {
                        let slice = &extracted_slices.slices[i];
                        let rect = slice.rect;
                        let rect_size = rect.size();

                        // Calculate vertex data for this item
                        let mut uv_offset_scale: Vec4;

                        // If a rect is specified, adjust UVs and the size of the quad
                        uv_offset_scale = Vec4::new(
                            rect.min.x / batch_image_size.x,
                            rect.max.y / batch_image_size.y,
                            rect_size.x / batch_image_size.x,
                            -rect_size.y / batch_image_size.y,
                        );

                        if extracted_sprite.flip_x {
                            uv_offset_scale.x += uv_offset_scale.z;
                            uv_offset_scale.z *= -1.0;
                        }
                        if extracted_sprite.flip_y {
                            uv_offset_scale.y += uv_offset_scale.w;
                            uv_offset_scale.w *= -1.0;
                        }

                        let transform = extracted_sprite.transform.affine()
                            * Affine3A::from_scale_rotation_translation(
                                slice.size.extend(1.0),
                                Quat::IDENTITY,
                                (slice.size * -Vec2::splat(0.5) + slice.offset).extend(0.0),
                            );

                        // Store the vertex data and add the item to the render phase
                        sprite_meta
                            .sprite_instance_buffer
                            .push(SpriteInstance::from(
                                &transform,
                                &extracted_sprite.color,
                                &uv_offset_scale,
                            ));

                        current_batch.as_mut().unwrap().get_mut().range.end += 1;
                        index += 1;
                    }
                }
            }
            transparent_phase.items[batch_item_index]
                .batch_range_mut()
                .end += 1;
        }
        sprite_meta
            .sprite_instance_buffer
            .write_buffer(&render_device, &render_queue);

        if sprite_meta.sprite_index_buffer.len() != 6 {
            sprite_meta.sprite_index_buffer.clear();

            // NOTE: This code is creating 6 indices pointing to 4 vertices.
            // The vertices form the corners of a quad based on their two least significant bits.
            // 10   11
            //
            // 00   01
            // The sprite shader can then use the two least significant bits as the vertex index.
            // The rest of the properties to transform the vertex positions and UVs (which are
            // implicit) are baked into the instance transform, and UV offset and scale.
            // See bevy_sprite_render/src/render/sprite.wgsl for the details.
            sprite_meta.sprite_index_buffer.push(2);
            sprite_meta.sprite_index_buffer.push(0);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(3);
            sprite_meta.sprite_index_buffer.push(2);

            sprite_meta
                .sprite_index_buffer
                .write_buffer(&render_device, &render_queue);
        }
    }
}
/// [`RenderCommand`] for sprite rendering.
pub type DrawSprite = (
    SetItemPipeline,
    SetSpriteViewBindGroup<0>,
    SetSpriteTextureBindGroup<1>,
    DrawSpriteBatch,
);

pub struct SetSpriteViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (Read<ViewUniformOffset>, Read<SpriteViewBindGroup>);
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        (view_uniform, sprite_view_bind_group): ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &sprite_view_bind_group.value, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}
pub struct SetSpriteTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteTextureBindGroup<I> {
    type Param = (SRes<ImageBindGroups>, SRes<SpriteBatches>);
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        (image_bind_groups, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(batch) = batches.get(&(view.retained_view_entity, item.entity())) else {
            return RenderCommandResult::Skip;
        };

        pass.set_bind_group(
            I,
            image_bind_groups
                .values
                .get(&batch.image_handle_id)
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

pub struct DrawSpriteBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawSpriteBatch {
    type Param = (SRes<SpriteMeta>, SRes<SpriteBatches>);
    type ViewQuery = Read<ExtractedView>;
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity: Option<()>,
        (sprite_meta, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_meta = sprite_meta.into_inner();
        let Some(batch) = batches.get(&(view.retained_view_entity, item.entity())) else {
            return RenderCommandResult::Skip;
        };

        pass.set_index_buffer(
            sprite_meta.sprite_index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.set_vertex_buffer(
            0,
            sprite_meta
                .sprite_instance_buffer
                .buffer()
                .unwrap()
                .slice(..),
        );
        pass.draw_indexed(0..6, 0, batch.range.clone());
        RenderCommandResult::Success
    }
}

/// Scales a texture to fit within a given quad size with keeping the aspect ratio.
fn apply_scaling(
    scaling_mode: ScalingMode,
    texture_size: Vec2,
    quad_size: &mut Vec2,
    quad_translation: &mut Vec2,
    uv_offset_scale: &mut Vec4,
) {
    let quad_ratio = quad_size.x / quad_size.y;
    let texture_ratio = texture_size.x / texture_size.y;
    let tex_quad_scale = texture_ratio / quad_ratio;
    let quad_tex_scale = quad_ratio / texture_ratio;

    match scaling_mode {
        ScalingMode::FillCenter => {
            if quad_ratio > texture_ratio {
                // offset texture to center by y coordinate
                uv_offset_scale.y += (uv_offset_scale.w - uv_offset_scale.w * tex_quad_scale) * 0.5;
                // sum up scales
                uv_offset_scale.w *= tex_quad_scale;
            } else {
                // offset texture to center by x coordinate
                uv_offset_scale.x += (uv_offset_scale.z - uv_offset_scale.z * quad_tex_scale) * 0.5;
                uv_offset_scale.z *= quad_tex_scale;
            };
        }
        ScalingMode::FillStart => {
            if quad_ratio > texture_ratio {
                uv_offset_scale.y += uv_offset_scale.w - uv_offset_scale.w * tex_quad_scale;
                uv_offset_scale.w *= tex_quad_scale;
            } else {
                uv_offset_scale.z *= quad_tex_scale;
            }
        }
        ScalingMode::FillEnd => {
            if quad_ratio > texture_ratio {
                uv_offset_scale.w *= tex_quad_scale;
            } else {
                uv_offset_scale.x += uv_offset_scale.z - uv_offset_scale.z * quad_tex_scale;
                uv_offset_scale.z *= quad_tex_scale;
            }
        }
        ScalingMode::FitCenter => {
            if texture_ratio > quad_ratio {
                // Scale based on width
                quad_size.y *= quad_tex_scale;
            } else {
                // Scale based on height
                quad_size.x *= tex_quad_scale;
            }
        }
        ScalingMode::FitStart => {
            if texture_ratio > quad_ratio {
                // The quad is scaled to match the image ratio, and the quad translation is adjusted
                // to start of the quad within the original quad size.
                let scale = Vec2::new(1.0, quad_tex_scale);
                let new_quad = *quad_size * scale;
                let offset = *quad_size - new_quad;
                *quad_translation = Vec2::new(0.0, -offset.y);
                *quad_size = new_quad;
            } else {
                let scale = Vec2::new(tex_quad_scale, 1.0);
                let new_quad = *quad_size * scale;
                let offset = *quad_size - new_quad;
                *quad_translation = Vec2::new(offset.x, 0.0);
                *quad_size = new_quad;
            }
        }
        ScalingMode::FitEnd => {
            if texture_ratio > quad_ratio {
                let scale = Vec2::new(1.0, quad_tex_scale);
                let new_quad = *quad_size * scale;
                let offset = *quad_size - new_quad;
                *quad_translation = Vec2::new(0.0, offset.y);
                *quad_size = new_quad;
            } else {
                let scale = Vec2::new(tex_quad_scale, 1.0);
                let new_quad = *quad_size * scale;
                let offset = *quad_size - new_quad;
                *quad_translation = Vec2::new(-offset.x, 0.0);
                *quad_size = new_quad;
            }
        }
    }
}
