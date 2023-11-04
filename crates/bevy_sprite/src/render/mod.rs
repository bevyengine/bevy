use std::ops::Range;

use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Mask, Masked, Sprite, SPRITE_SHADER_HANDLE,
};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_core_pipeline::{
    core_2d::Transparent2d,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_math::{Affine3A, Quat, Rect, Vec2, Vec4};
use bevy_render::{
    color::Color,
    render_asset::RenderAssets,
    render_phase::{
        DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
        TrackedRenderPass,
    },
    render_resource::{BindGroupEntries, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{
        BevyDefault, DefaultImageSampler, GpuImage, Image, ImageSampler, TextureFormatPixelInfo,
    },
    view::{
        ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms,
        ViewVisibility, VisibleEntities,
    },
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{EntityHashMap, FloatOrd, HashMap};
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

#[derive(Resource)]
pub struct SpritePipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
    mask_material_layout: BindGroupLayout,
    mask_uniform_layout: BindGroupLayout,
    pub dummy_white_gpu_image: GpuImage,
}

#[derive(Default, Clone, ShaderType)]
pub struct MaskUniform {
    threshold: f32,
    /// WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl")]
    _padding_a: f32,
    #[cfg(feature = "webgl")]
    _padding_b: f32,
    #[cfg(feature = "webgl")]
    _padding_c: f32,
}

impl PartialEq for MaskUniform {
    fn eq(&self, other: &Self) -> bool {
        FloatOrd(self.threshold) == FloatOrd(other.threshold)
    }
}

impl Eq for MaskUniform {}

#[derive(Default, Resource)]
pub struct MaskUniforms {
    pub uniforms: DynamicUniformBuffer<MaskUniform>,
}

impl FromWorld for SpritePipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
            label: Some("sprite_view_layout"),
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("sprite_material_layout"),
        });

        let mask_material_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("sprite_mask_material_layout"),
            });

        let mask_uniform_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(MaskUniform::min_size()),
                    },
                    count: None,
                }],
                label: Some("sprite_mask_uniform_layout"),
            });

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
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &image.data,
                ImageDataLayout {
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
                size: image.size_f32(),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        SpritePipeline {
            view_layout,
            material_layout,
            mask_material_layout,
            mask_uniform_layout,
            dummy_white_gpu_image,
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    pub struct SpritePipelineKey: u32 {
        const NONE                              = 0;
        const COLORED                           = (1 << 0);
        const HDR                               = (1 << 1);
        const TONEMAP_IN_SHADER                 = (1 << 2);
        const DEBAND_DITHER                     = (1 << 3);
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
        const MASK_RESERVED_BITS                = Self::MASK_MASK_BITS << Self::MASK_SHIFT_BITS;
        const MASK_ENABLED                      = 1 << Self::MASK_SHIFT_BITS;
        const MASK_THRESHOLD                    = (1 << 1) << Self::MASK_SHIFT_BITS;
    }
}

impl SpritePipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();

    const TONEMAP_METHOD_MASK_BITS: u32 = 0b111;
    const TONEMAP_METHOD_SHIFT_BITS: u32 =
        Self::MSAA_SHIFT_BITS - Self::TONEMAP_METHOD_MASK_BITS.count_ones();

    const MASK_MASK_BITS: u32 = 0b11;
    const MASK_SHIFT_BITS: u32 =
        Self::TONEMAP_METHOD_SHIFT_BITS - Self::MASK_MASK_BITS.count_ones();

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
    pub const fn from_colored(colored: bool) -> Self {
        if colored {
            SpritePipelineKey::COLORED
        } else {
            SpritePipelineKey::NONE
        }
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

        #[cfg(feature = "webgl")]
        shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());

        if key.contains(SpritePipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

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

        let use_mask = key.contains(SpritePipelineKey::MASK_ENABLED);
        let use_mask_threshold = key.contains(SpritePipelineKey::MASK_THRESHOLD);

        if use_mask {
            shader_defs.push("MASK".into());

            if use_mask_threshold {
                shader_defs.push("MASK_THRESHOLD".into());
            }
        }

        let format = match key.contains(SpritePipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };

        let mut vertex_buffer_array_stride = 80;
        let mut vertex_buffer_attributes = vec![
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
        ];

        if use_mask {
            vertex_buffer_array_stride += 64;
            vertex_buffer_attributes.append(&mut vec![
                // @location(5) i_mask_model_transpose_col0: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 80,
                    shader_location: 5,
                },
                // @location(6) i_mask_model_transpose_col1: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 96,
                    shader_location: 6,
                },
                // @location(7) i_mask_model_transpose_col2: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 112,
                    shader_location: 7,
                },
                // @location(8) i_mask_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 128,
                    shader_location: 8,
                },
            ]);
        }

        let mut pipeline_layout = vec![self.view_layout.clone(), self.material_layout.clone()];

        if use_mask {
            pipeline_layout.push(self.mask_material_layout.clone());

            if use_mask_threshold {
                pipeline_layout.push(self.mask_uniform_layout.clone());
            }
        }

        let instance_rate_vertex_buffer_layout = VertexBufferLayout {
            array_stride: vertex_buffer_array_stride,
            step_mode: VertexStepMode::Instance,
            attributes: vertex_buffer_attributes,
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SPRITE_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: pipeline_layout,
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("sprite_pipeline".into()),
            push_constant_ranges: Vec::new(),
        }
    }
}

fn calculate_transform(
    image_size: &Vec2,
    custom_size: &Option<Vec2>,
    rect: &Option<Rect>,
    transform: &GlobalTransform,
    anchor: &Vec2,
) -> Affine3A {
    // By default, the size of the quad is the size of the texture, but `rect` or `custom_size` will overwrite
    let quad_size = custom_size.unwrap_or_else(|| rect.map(|r| r.size()).unwrap_or(*image_size));

    transform.affine()
        * Affine3A::from_scale_rotation_translation(
            quad_size.extend(1.0),
            Quat::IDENTITY,
            (quad_size * (-*anchor - Vec2::splat(0.5))).extend(0.0),
        )
}

fn calculate_uv_offset_scale(
    image_size: &Vec2,
    rect: &Option<Rect>,
    flip_x: bool,
    flip_y: bool,
) -> Vec4 {
    // If a rect is specified, adjust UVs
    let mut uv_offset_scale = if let Some(rect) = rect {
        let rect_size = rect.size();
        Vec4::new(
            rect.min.x / image_size.x,
            rect.max.y / image_size.y,
            rect_size.x / image_size.x,
            -rect_size.y / image_size.y,
        )
    } else {
        Vec4::new(0.0, 1.0, 1.0, -1.0)
    };

    if flip_x {
        uv_offset_scale.x += uv_offset_scale.z;
        uv_offset_scale.z *= -1.0;
    }
    if flip_y {
        uv_offset_scale.y += uv_offset_scale.w;
        uv_offset_scale.w *= -1.0;
    }

    uv_offset_scale
}

pub struct ExtractedSprite {
    pub transform: GlobalTransform,
    pub color: Color,
    /// Select an area of the texture
    pub rect: Option<Rect>,
    /// Change the on-screen size of the sprite
    pub custom_size: Option<Vec2>,
    /// Asset ID of the [`Image`] of this sprite
    /// PERF: storing an `AssetId` instead of `Handle<Image>` enables some optimizations (`ExtractedSprite` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: AssetId<Image>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub anchor: Vec2,
    /// For cases where additional ExtractedSprites are created during extraction, this stores the
    /// entity that caused that creation for use in determining visibility.
    pub original_entity: Option<Entity>,

    pub mask: Option<Entity>,
}

impl ExtractedSprite {
    fn calculate_transform(&self, image_size: &Vec2) -> Affine3A {
        calculate_transform(
            image_size,
            &self.custom_size,
            &self.rect,
            &self.transform,
            &self.anchor,
        )
    }

    fn calculate_uv_offset_scale(&self, image_size: &Vec2) -> Vec4 {
        calculate_uv_offset_scale(image_size, &self.rect, self.flip_x, self.flip_y)
    }
}

pub struct ExtractedMask {
    pub transform: GlobalTransform,
    /// Select an area of the texture
    pub rect: Option<Rect>,
    /// Change the on-screen size of the mask
    pub custom_size: Option<Vec2>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub anchor: Vec2,

    pub image_handle_id: AssetId<Image>,
    pub threshold: Option<f32>,
    pub uniform_offset: Option<u32>,
}

impl ExtractedMask {
    pub fn sprite_pipeline_key(&self) -> SpritePipelineKey {
        let threshold_key = match &self.threshold {
            Some(_) => SpritePipelineKey::MASK_THRESHOLD,
            None => SpritePipelineKey::NONE,
        };

        SpritePipelineKey::MASK_ENABLED | threshold_key
    }
}

impl ExtractedMask {
    fn calculate_transform(&self, image_size: &Vec2) -> Affine3A {
        calculate_transform(
            image_size,
            &self.custom_size,
            &self.rect,
            &self.transform,
            &self.anchor,
        )
    }

    fn calculate_uv_offset_scale(&self, image_size: &Vec2) -> Vec4 {
        calculate_uv_offset_scale(image_size, &self.rect, self.flip_x, self.flip_y)
    }
}

#[derive(Resource, Default)]
pub struct ExtractedSprites {
    pub sprites: EntityHashMap<Entity, ExtractedSprite>,
    pub masks: EntityHashMap<Entity, Option<ExtractedMask>>,
    pub mask_uniform_count: usize,
}

impl ExtractedSprites {
    fn clear(&mut self) {
        self.sprites.clear();
        self.masks.clear();
        self.mask_uniform_count = 0;
    }
}

#[derive(Resource, Default)]
pub struct SpriteAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub fn extract_sprite_events(
    mut events: ResMut<SpriteAssetEvents>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
    let SpriteAssetEvents { ref mut images } = *events;
    images.clear();

    for event in image_events.read() {
        images.push(*event);
    }
}

pub fn extract_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    sprite_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &Sprite,
            &GlobalTransform,
            &Handle<Image>,
            Option<&Masked>,
        )>,
    >,
    atlas_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &TextureAtlasSprite,
            &GlobalTransform,
            &Handle<TextureAtlas>,
            Option<&Masked>,
        )>,
    >,
    mask_query: Extract<Query<(&ViewVisibility, &Mask, &GlobalTransform)>>,
) {
    extracted_sprites.clear();

    let extract_mask = |extracted_sprites: &mut ExtractedSprites, masked: Option<&Masked>| {
        let mask = masked.map(|m| m.mask);

        if let Some(mask) = mask {
            extracted_sprites.masks.entry(mask).or_insert_with(|| {
                let extracted_mask = match mask_query.get(mask) {
                    Ok((view_visibility, mask, transform)) => {
                        if !view_visibility.get() {
                            None
                        } else {
                            Some(ExtractedMask {
                                transform: *transform,
                                rect: mask.rect,
                                custom_size: mask.custom_size,
                                flip_x: mask.flip_x,
                                flip_y: mask.flip_y,
                                anchor: mask.anchor.as_vec(),
                                image_handle_id: mask.image.id(),
                                threshold: mask.threshold,
                                uniform_offset: None,
                            })
                        }
                    }
                    // The Masked does not point to an Entity with a Mask.
                    // Store this failure instead of querying again.
                    Err(_) => None,
                };

                extracted_sprites.mask_uniform_count += extracted_mask.is_some() as usize;

                extracted_mask
            });
        }

        mask
    };

    for (entity, view_visibility, sprite, transform, handle, masked) in sprite_query.iter() {
        if !view_visibility.get() {
            continue;
        }

        let mask = extract_mask(&mut extracted_sprites, masked);

        // PERF: we don't check in this function that the `Image` asset is ready, since it should be in most cases and hashing the handle is expensive
        extracted_sprites.sprites.insert(
            entity,
            ExtractedSprite {
                color: sprite.color,
                transform: *transform,
                rect: sprite.rect,
                // Pass the custom size
                custom_size: sprite.custom_size,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: handle.id(),
                anchor: sprite.anchor.as_vec(),
                original_entity: None,

                mask,
            },
        );
    }
    for (entity, view_visibility, atlas_sprite, transform, texture_atlas_handle, masked) in
        atlas_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }
        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            let rect = Some(
                *texture_atlas
                    .textures
                    .get(atlas_sprite.index)
                    .unwrap_or_else(|| {
                        panic!(
                            "Sprite index {:?} does not exist for texture atlas handle {:?}.",
                            atlas_sprite.index,
                            texture_atlas_handle.id(),
                        )
                    }),
            );

            let mask = extract_mask(&mut extracted_sprites, masked);

            extracted_sprites.sprites.insert(
                entity,
                ExtractedSprite {
                    color: atlas_sprite.color,
                    transform: *transform,
                    // Select the area in the texture atlas
                    rect,
                    // Pass the custom size
                    custom_size: atlas_sprite.custom_size,
                    flip_x: atlas_sprite.flip_x,
                    flip_y: atlas_sprite.flip_y,
                    image_handle_id: texture_atlas.texture.id(),
                    anchor: atlas_sprite.anchor.as_vec(),
                    original_entity: None,

                    mask,
                },
            );
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
    fn from(transform: &Affine3A, color: &Color, uv_offset_scale: &Vec4) -> Self {
        let transpose_model_3x3 = transform.matrix3.transpose();
        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(transform.translation.x),
                transpose_model_3x3.y_axis.extend(transform.translation.y),
                transpose_model_3x3.z_axis.extend(transform.translation.z),
            ],
            i_color: color.as_linear_rgba_f32(),
            i_uv_offset_scale: uv_offset_scale.to_array(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct MaskedSpriteInstance {
    pub sprite: SpriteInstance,
    // Affine 4x3 transposed to 3x4
    pub i_mask_model_transpose: [Vec4; 3],
    pub i_mask_uv_offset_scale: [f32; 4],
}

impl MaskedSpriteInstance {
    #[inline]
    fn from(
        sprite_instance: SpriteInstance,
        mask_transform: &Affine3A,
        mask_uv_offset_scale: &Vec4,
    ) -> Self {
        let transpose_model_3x3 = mask_transform.matrix3.transpose();
        Self {
            sprite: sprite_instance,
            i_mask_model_transpose: [
                transpose_model_3x3
                    .x_axis
                    .extend(mask_transform.translation.x),
                transpose_model_3x3
                    .y_axis
                    .extend(mask_transform.translation.y),
                transpose_model_3x3
                    .z_axis
                    .extend(mask_transform.translation.z),
            ],
            i_mask_uv_offset_scale: mask_uv_offset_scale.to_array(),
        }
    }
}

#[derive(Resource)]
pub struct SpriteMeta {
    view_bind_group: Option<BindGroup>,
    sprite_index_buffer: BufferVec<u32>,
    sprite_instance_buffer: BufferVec<SpriteInstance>,
    masked_sprite_instance_buffer: BufferVec<MaskedSpriteInstance>,
}

impl SpriteMeta {
    fn clear(&mut self) {
        self.view_bind_group = None;
        self.sprite_index_buffer.clear();
        self.sprite_instance_buffer.clear();
        self.masked_sprite_instance_buffer.clear();
    }
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            view_bind_group: None,
            sprite_index_buffer: BufferVec::<u32>::new(BufferUsages::INDEX),
            sprite_instance_buffer: BufferVec::<SpriteInstance>::new(BufferUsages::VERTEX),
            masked_sprite_instance_buffer: BufferVec::<MaskedSpriteInstance>::new(
                BufferUsages::VERTEX,
            ),
        }
    }
}

#[derive(Component, PartialEq, Eq, Clone)]
pub struct SpriteBatch {
    image_handle_id: AssetId<Image>,
    range: Range<u32>,
    mask: Option<MaskBatch>,
}

#[derive(PartialEq, Eq, Clone)]
pub struct MaskBatch {
    mask_handle_id: AssetId<Image>,
    uniform_offset: Option<u32>,
}

#[derive(Resource, Default)]
pub struct ImageBindGroups {
    values: HashMap<AssetId<Image>, BindGroup>,
    mask_values: HashMap<AssetId<Image>, BindGroup>,
    mask_uniforms_value: Option<BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_sprites(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    sprite_pipeline: Res<SpritePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SpritePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    extracted_sprites: Res<ExtractedSprites>,
    mut views: Query<(
        &mut RenderPhase<Transparent2d>,
        &VisibleEntities,
        &ExtractedView,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
) {
    let msaa_key = SpritePipelineKey::from_msaa_samples(msaa.samples());

    let draw_sprite_function = draw_functions.read().id::<DrawSprite>();

    for (mut transparent_phase, visible_entities, view, tonemapping, dither) in &mut views {
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

        view_entities.clear();
        view_entities.extend(visible_entities.entities.iter().map(|e| e.index() as usize));

        transparent_phase
            .items
            .reserve(extracted_sprites.sprites.len());

        for (entity, extracted_sprite) in extracted_sprites.sprites.iter() {
            let index = extracted_sprite.original_entity.unwrap_or(*entity).index();

            if !view_entities.contains(index as usize) {
                continue;
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_sprite.transform.translation().z);

            let mut specialize_pipeline = |view_key: SpritePipelineKey| {
                pipelines.specialize(&pipeline_cache, &sprite_pipeline, view_key)
            };

            let mask_key = extracted_sprite
                .mask
                .map(|m| extracted_sprites.masks.get(&m).map(Option::as_ref))
                .flatten()
                .flatten()
                .map(|em| em.sprite_pipeline_key())
                .unwrap_or(SpritePipelineKey::NONE);

            let color_key = SpritePipelineKey::from_colored(extracted_sprite.color != Color::WHITE);

            let pipeline = specialize_pipeline(view_key | mask_key | color_key);

            // Add the item to the render phase
            transparent_phase.add(Transparent2d {
                draw_function: draw_sprite_function,
                pipeline,
                entity: *entity,
                sort_key,
                // batch_range and dynamic_offset will be calculated in prepare_sprites
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}

pub fn prepare_mask_uniforms(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut mask_uniforms: ResMut<MaskUniforms>,
    mut extracted_sprites: ResMut<ExtractedSprites>,
) {
    if extracted_sprites.mask_uniform_count > 0 {
        let Some(mut writer) =
            mask_uniforms
                .uniforms
                .get_writer(extracted_sprites.mask_uniform_count, &render_device, &render_queue)
        else {
            return;
        };

        let mut last_uniform = None;
        let mut last_uniform_offset = 0;

        for mask in extracted_sprites
            .masks
            .iter_mut()
            .filter_map(|(_, mask)| mask.as_mut())
        {
            if let Some(threshold) = mask.threshold {
                let uniform = MaskUniform {
                    threshold,
                    ..Default::default()
                };
                if last_uniform.as_ref() != Some(&uniform) {
                    last_uniform_offset = writer.write(&uniform);
                    last_uniform = Some(uniform);
                }
                mask.uniform_offset = Some(last_uniform_offset);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_sprites(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta>,
    view_uniforms: Res<ViewUniforms>,
    mask_uniforms: Res<MaskUniforms>,
    sprite_pipeline: Res<SpritePipeline>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    extracted_sprites: Res<ExtractedSprites>,
    mut phases: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<SpriteAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added {..} |
            // images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
                image_bind_groups.mask_values.remove(id);
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, SpriteBatch)> = Vec::with_capacity(*previous_len);

        // Clear the sprite instances
        sprite_meta.clear();

        sprite_meta.view_bind_group = Some(render_device.create_bind_group(
            "sprite_view_bind_group",
            &sprite_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        if extracted_sprites.mask_uniform_count > 0 {
            image_bind_groups
                .mask_uniforms_value
                .get_or_insert_with(|| {
                    let uniform_binding = mask_uniforms.uniforms.binding().unwrap();

                    render_device.create_bind_group(&BindGroupDescriptor {
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: uniform_binding,
                        }],
                        label: Some("sprite_mask_uniform_bind_group"),
                        layout: &sprite_pipeline.mask_uniform_layout,
                    })
                });
        }

        // Index buffer indices
        let mut unmasked_index = 0;
        let mut masked_index = 0;

        let image_bind_groups = &mut *image_bind_groups;

        for mut transparent_phase in &mut phases {
            let mut batch_item_index = 0;
            let mut batch_image_size = Vec2::ZERO;
            let mut batch_image_handle = AssetId::invalid();

            let mut batch_mask_image_size = Vec2::ZERO;
            // The mask potentially controls several uniforms, so batch based on the entity instead of just the image
            let mut batch_mask_handle = None;
            let mut batch_mask_uniform_offset = None;

            // Iterate through the phase items and detect when successive sprites that can be batched.
            // Spawn an entity with a `SpriteBatch` component for each possible batch.
            // Compatible items share the same entity.
            for item_index in 0..transparent_phase.items.len() {
                let item = &transparent_phase.items[item_index];
                let Some(extracted_sprite) = extracted_sprites.sprites.get(&item.entity) else {
                    // If there is a phase item that is not a sprite, then we must start a new
                    // batch to draw the other phase item(s) and to respect draw order. This can be
                    // done by invalidating the batch_image_handle
                    batch_image_handle = AssetId::invalid();
                    continue;
                };

                let batch_image_changed = batch_image_handle != extracted_sprite.image_handle_id;

                if batch_image_changed {
                    let Some(gpu_image) = gpu_images.get(extracted_sprite.image_handle_id) else {
                        continue;
                    };

                    batch_image_size = Vec2::new(gpu_image.size.x, gpu_image.size.y);
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
                }

                let extracted_mask = extracted_sprite
                    .mask
                    .as_ref()
                    .map(|e| extracted_sprites.masks.get(e).map(Option::as_ref))
                    .flatten()
                    .flatten();
                let mask_asset = extracted_mask.map(|m| m.image_handle_id);

                let batch_mask_changed = batch_mask_handle != mask_asset;

                if batch_mask_changed {
                    if let (Some(extracted_mask), Some(mask_asset)) = (extracted_mask, mask_asset) {
                        let Some(gpu_image) = gpu_images.get(extracted_mask.image_handle_id) else {
                            continue;
                        };

                        batch_mask_image_size = Vec2::new(gpu_image.size.x, gpu_image.size.y);

                        image_bind_groups
                            .mask_values
                            .entry(mask_asset)
                            .or_insert_with(|| {
                                render_device.create_bind_group(&BindGroupDescriptor {
                                    entries: &[
                                        BindGroupEntry {
                                            binding: 0,
                                            resource: BindingResource::TextureView(
                                                &gpu_image.texture_view,
                                            ),
                                        },
                                        BindGroupEntry {
                                            binding: 1,
                                            resource: BindingResource::Sampler(&gpu_image.sampler),
                                        },
                                    ],
                                    label: Some("sprite_mask_material_bind_group"),
                                    layout: &sprite_pipeline.mask_material_layout,
                                })
                            });
                    }

                    batch_mask_handle = mask_asset;
                }

                let mask_uniform_offset = extracted_mask.map(|m| m.uniform_offset).flatten();
                let batch_mask_uniform_changed = batch_mask_uniform_offset != mask_uniform_offset;
                if batch_mask_uniform_changed {
                    batch_mask_uniform_offset = mask_uniform_offset;
                }

                let sprite_transform = extracted_sprite.calculate_transform(&batch_image_size);

                let sprite_instance = SpriteInstance::from(
                    &sprite_transform,
                    &extracted_sprite.color,
                    &extracted_sprite.calculate_uv_offset_scale(&batch_image_size),
                );

                // Store the vertex data and add the item to the render phase
                let index = if let Some(extracted_mask) = extracted_mask {
                    let masked_sprite_instance = MaskedSpriteInstance::from(
                        sprite_instance,
                        &(extracted_mask
                            .calculate_transform(&batch_mask_image_size)
                            .inverse()
                            * sprite_transform),
                        &extracted_mask.calculate_uv_offset_scale(&batch_mask_image_size),
                    );

                    sprite_meta
                        .masked_sprite_instance_buffer
                        .push(masked_sprite_instance);

                    &mut masked_index
                } else {
                    sprite_meta.sprite_instance_buffer.push(sprite_instance);

                    &mut unmasked_index
                };

                if batch_image_changed || batch_mask_changed || batch_mask_uniform_changed {
                    batch_item_index = item_index;

                    let mask_batch = extracted_mask.map(|em| MaskBatch {
                        mask_handle_id: em.image_handle_id,
                        uniform_offset: em.uniform_offset,
                    });

                    batches.push((
                        item.entity,
                        SpriteBatch {
                            image_handle_id: batch_image_handle,
                            range: *index..*index,
                            mask: mask_batch,
                        },
                    ));
                }

                transparent_phase.items[batch_item_index]
                    .batch_range_mut()
                    .end += 1;
                batches.last_mut().unwrap().1.range.end += 1;
                *index += 1;
            }
        }
        sprite_meta
            .sprite_instance_buffer
            .write_buffer(&render_device, &render_queue);

        sprite_meta
            .masked_sprite_instance_buffer
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
            // See bevy_sprite/src/render/sprite.wgsl for the details.
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

        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
}

pub type DrawSprite = (
    SetItemPipeline,
    SetSpriteViewBindGroup<0>,
    SetSpriteTextureBindGroup<1>,
    SetSpriteMaskTextureBindGroup<2>,
    SetSpriteMaskUniformsBindGroup<3>,
    DrawSpriteBatch,
);

pub struct SetSpriteViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteViewBindGroup<I> {
    type Param = SRes<SpriteMeta>;
    type ViewWorldQuery = Read<ViewUniformOffset>;
    type ItemWorldQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'_ ViewUniformOffset,
        _entity: (),
        sprite_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            sprite_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}
pub struct SetSpriteTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteTextureBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();

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

pub struct SetSpriteMaskTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteMaskTextureBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();

        if let Some(mask_batch) = &batch.mask {
            pass.set_bind_group(
                I,
                image_bind_groups
                    .mask_values
                    .get(&mask_batch.mask_handle_id)
                    .unwrap(),
                &[],
            );
        }

        RenderCommandResult::Success
    }
}

pub struct SetSpriteMaskUniformsBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteMaskUniformsBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();

        if let Some(mask_batch) = &batch.mask {
            if let Some(uniform_offset) = mask_batch.uniform_offset {
                pass.set_bind_group(
                    I,
                    image_bind_groups.mask_uniforms_value.as_ref().unwrap(),
                    &[uniform_offset],
                );
            }
        }

        RenderCommandResult::Success
    }
}

pub struct DrawSpriteBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawSpriteBatch {
    type Param = SRes<SpriteMeta>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch,
        sprite_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_meta = sprite_meta.into_inner();
        pass.set_index_buffer(
            sprite_meta.sprite_index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        let buffer = if batch.mask.is_some() {
            sprite_meta.masked_sprite_instance_buffer.buffer()
        } else {
            sprite_meta.sprite_instance_buffer.buffer()
        };
        pass.set_vertex_buffer(0, buffer.unwrap().slice(..));
        pass.draw_indexed(0..6, 0, batch.range.clone());
        RenderCommandResult::Success
    }
}
