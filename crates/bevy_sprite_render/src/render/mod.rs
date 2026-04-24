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
use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{Affine3A, FloatOrd, Quat, Rect, Vec2, Vec4};
use bevy_mesh::VertexBufferLayout;
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedCamera,
    view::{RenderVisibleEntities, RetainedViewEntity},
};
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
    texture::{FallbackImage, GpuImage},
    view::{
        texture_format_from_code, texture_format_to_code, ExtractedView, Msaa, ViewUniform,
        ViewUniformOffset, ViewUniforms,
    },
    Extract,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_sprite::{Anchor, Sprite, SpriteScalingMode};
// Subpixel text tuning resources live in `bevy_text` (consolidated in
// phase-05 of spec/0013). `bevy_text` is an optional dependency of
// `bevy_sprite_render`; the extraction system that copies the resources
// into the uniform is feature-gated. The uniform struct itself and the
// view bind-group layout entry are compiled unconditionally so the
// `SpritePipelineKey::SUBPIXEL` variant always has a consistent layout.
#[cfg(feature = "bevy_text")]
use bevy_text::{SubpixelLcdLayout, SubpixelTextSettings};
use bevy_transform::components::GlobalTransform;
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

#[derive(Resource)]
pub struct SpritePipeline {
    pub view_layout: BindGroupLayoutDescriptor,
    pub material_layout: BindGroupLayoutDescriptor,
    pub shader: Handle<Shader>,
}

pub fn init_sprite_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
    // Binding 3 holds the `SubpixelTextUniforms` consumed by the
    // `fragment_subpixel` entry point. Declared on every sprite pipeline
    // variant so the view bind-group layout is shared between the standard
    // sprite path and the subpixel text path. Non-subpixel fragment entries
    // don't reference the binding; naga/wgpu tolerate unused bindings as
    // long as the layout matches. Mirrors
    // `bevy_ui_render::pipeline::init_ui_pipeline` (phase-04 of spec/0013).
    let view_layout = BindGroupLayoutDescriptor::new(
        "sprite_view_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ViewUniform>(true),
                tonemapping_lut_entries[0].visibility(ShaderStages::FRAGMENT),
                tonemapping_lut_entries[1].visibility(ShaderStages::FRAGMENT),
                uniform_buffer::<SubpixelTextUniforms>(false).visibility(ShaderStages::FRAGMENT),
            ),
        ),
    );

    let material_layout = BindGroupLayoutDescriptor::new(
        "sprite_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    commands.insert_resource(SpritePipeline {
        view_layout,
        material_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "sprite.wgsl"),
    });
}

/// GPU-facing form of [`bevy_text::SubpixelTextSettings`] and
/// [`bevy_text::SubpixelLcdLayout`], bound as `@group(0) @binding(3)` of the
/// sprite view bind group on every pipeline variant.
///
/// Duplicated from `bevy_ui_render::SubpixelTextUniforms` — each render crate
/// owns its own `ShaderType`-derived uniform struct because `ShaderType`
/// lives in `bevy_render` (which `bevy_text` deliberately does not depend
/// on), so the struct can't be hoisted to the shared crate. The layout is
/// identical byte-for-byte; both structs serialise from the same
/// [`SubpixelTextSettings`] + [`SubpixelLcdLayout`] inputs. A future
/// consolidation spec can merge the two.
///
/// `std140` layout notes (match `bevy_ui_render::SubpixelTextUniforms`):
/// - `enhanced_contrast: f32` (offset 0) + `layout_flags: u32` (offset 4)
///   pack into the first 8 bytes of the leading 16-byte slot.
/// - `_pad: Vec2` (offset 8) fills the rest of that 16-byte slot so the
///   trailing `vec4<f32> gamma_ratios` lands on its required 16-byte
///   boundary.
///
/// Total: 32 bytes.
///
/// Populated each frame by [`prepare_sprite_view_bind_groups`] from the
/// main-world [`SubpixelTextSettings`] / [`SubpixelLcdLayout`] resources
/// when the `bevy_text` feature is enabled. When the feature is off, the
/// buffer holds [`SubpixelTextUniforms::default`] permanently — the
/// subpixel fragment path still compiles but is never reached because no
/// sprite sets [`ExtractedSprite::subpixel`] to `true`.
#[derive(ShaderType, Clone, Copy, Debug)]
pub struct SubpixelTextUniforms {
    /// Enhanced-contrast factor (GPUI default `0.5`). Scales Skia's light-
    /// on-dark contrast ramp inside `fragment_subpixel`.
    pub enhanced_contrast: f32,
    /// LCD subpixel layout discriminant. Matches the constants in
    /// `sprite.wgsl` (`SUBPIXEL_LAYOUT_HORIZONTAL_RGB = 0`, etc.) and
    /// [`bevy_text::SubpixelLcdLayout::pack_u32`].
    pub layout_flags: u32,
    /// Explicit padding so `gamma_ratios` lands on a std140 16-byte
    /// boundary. Must match the shader's `_pad: vec2<f32>`.
    pub _pad: Vec2,
    /// Four-element gamma-correction table row, precomputed from GPUI's
    /// `GAMMA_INCORRECT_TARGET_RATIOS` at gamma = 1.8. See
    /// `zed/crates/gpui/src/platform.rs::get_gamma_correction_ratios`.
    pub gamma_ratios: Vec4,
}

impl Default for SubpixelTextUniforms {
    fn default() -> Self {
        // Must match `SubpixelTextSettings::default()` +
        // `SubpixelLcdLayout::default()` so the initial bind-group value is
        // consistent with the tuning resources before the first prepare
        // system run. `enhanced_contrast = 0.5` is GPUI's
        // `RenderingParameters::new()`; the gamma ratios are the gamma-1.8
        // row of `GAMMA_INCORRECT_TARGET_RATIOS`. `layout_flags = 0` is
        // `HorizontalRgb` — the identity swizzle in
        // `swizzle_subpixel_atlas`.
        Self {
            enhanced_contrast: 0.5,
            layout_flags: 0,
            _pad: Vec2::ZERO,
            gamma_ratios: Vec4::new(0.14746, -0.89481, 1.47021, -0.32474),
        }
    }
}

/// Populate the [`bevy_text::SubpixelCapable`] resource from the active
/// render adapter's wgpu feature set at `RenderStartup`. Reads
/// [`WgpuFeatures::DUAL_SOURCE_BLENDING`] — the feature the subpixel sprite
/// fragment entry (`fragment_subpixel` in `sprite.wgsl`) requires for its
/// `@blend_src(1)` output.
///
/// On adapters without DSB support, [`crate::extract_text2d_sprite`] forces
/// subpixel-smoothed glyph batches to the grayscale pipeline variant — no
/// panic, no shader error, just a silent visual degradation to approximate
/// grayscale AA (only the `.r` channel of the RGBA coverage atlas is used).
///
/// Idempotent with `bevy_ui_render::init_ui_subpixel_capability`: both
/// systems read the same underlying adapter feature set and therefore
/// produce the same [`bevy_text::SubpixelCapable`] value, so last-writer-
/// wins is correct regardless of the render sub-app startup order.
///
/// Only compiled when the `bevy_text` feature is enabled — without
/// `Text2d` there are no subpixel-flagged sprites, so the capability flag
/// isn't needed by any system in this crate.
#[cfg(feature = "bevy_text")]
pub fn init_sprite_subpixel_capability(mut commands: Commands, render_device: Res<RenderDevice>) {
    let supported = render_device
        .features()
        .contains(WgpuFeatures::DUAL_SOURCE_BLENDING);
    commands.insert_resource(bevy_text::SubpixelCapable(supported));
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    // MSAA uses the highest 3 bits for the MSAA log2(sample count) to support up to 128x MSAA.
    //
    // Bit layout:
    //   0        TONEMAP_IN_SHADER
    //   1        DEBAND_DITHER
    //   2        SRGB_COMPOSITING
    //   3        OKLAB_COMPOSITING
    //   4..=8    COLOR_TARGET_FORMAT  (5 bits, `COLOR_TARGET_FORMAT_MASK_BITS`)
    //   9        SUBPIXEL             (first unused bit after the color target reserved range)
    //   10..=24  (free for future use)
    //   25..=28  TONEMAP_METHOD       (4 bits)
    //   29..=31  MSAA                 (3 bits)
    pub struct SpritePipelineKey: u32 {
        const NONE                              = 0;
        const TONEMAP_IN_SHADER                 = 1 << 0;
        const DEBAND_DITHER                     = 1 << 1;
        const SRGB_COMPOSITING                  = 1 << 2;
        const OKLAB_COMPOSITING                 = 1 << 3;
        const COLOR_TARGET_FORMAT_RESERVED_BITS = Self::COLOR_TARGET_FORMAT_MASK_BITS << Self::COLOR_TARGET_FORMAT_SHIFT_BITS;
        /// Selects the RGB subpixel text pipeline variant (phase-06 of
        /// spec/0013). When set, [`SpritePipeline::specialize`] emits the
        /// dual-source-blend variant (`fragment_subpixel` entry point,
        /// `SUBPIXEL` `shader_def`, `Src1` / `OneMinusSrc1` color blend).
        /// Only flipped by [`crate::extract_text2d_sprite`] for glyph
        /// batches whose source section used
        /// [`FontSmoothing::SubpixelAntiAliased`](bevy_text::FontSmoothing::SubpixelAntiAliased)
        /// and only when the adapter advertises
        /// [`wgpu::Features::DUAL_SOURCE_BLENDING`](https://docs.rs/wgpu/latest/wgpu/struct.Features.html#associatedconstant.DUAL_SOURCE_BLENDING)
        /// (see [`bevy_text::SubpixelCapable`]).
        const SUBPIXEL                          = 1 << 9;
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
        const TONEMAP_METHOD_PBR_NEUTRAL        = 8 << Self::TONEMAP_METHOD_SHIFT_BITS;

    }
}

impl SpritePipelineKey {
    const COLOR_TARGET_FORMAT_MASK_BITS: u32 = bevy_render::view::COLOR_TARGET_FORMAT_MASK_BITS;
    const COLOR_TARGET_FORMAT_SHIFT_BITS: u32 = 4;
    const MSAA_MASK_BITS: u32 = 0b111;
    const MSAA_SHIFT_BITS: u32 = 32 - Self::MSAA_MASK_BITS.count_ones();
    const TONEMAP_METHOD_MASK_BITS: u32 = 0b1111;
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

    /// Create a pipeline key from the view's color target format.
    #[inline]
    pub fn from_target_format(format: TextureFormat) -> Self {
        let code = texture_format_to_code(format)
            .expect("Texture format is not supported by the pipeline") as u32;
        Self::from_bits_retain(
            (code & Self::COLOR_TARGET_FORMAT_MASK_BITS) << Self::COLOR_TARGET_FORMAT_SHIFT_BITS,
        )
    }

    /// Color target format of the main pass for this pipeline key.
    #[inline]
    pub fn target_format(&self) -> TextureFormat {
        let code = ((self.bits() >> Self::COLOR_TARGET_FORMAT_SHIFT_BITS)
            & Self::COLOR_TARGET_FORMAT_MASK_BITS) as u8;
        texture_format_from_code(code)
            .expect("Unknown bits in `COLOR_TARGET_FORMAT_MASK_BITS` of the pipeline key")
    }
}

impl SpecializedRenderPipeline for SpritePipeline {
    type Key = SpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let subpixel = key.contains(SpritePipelineKey::SUBPIXEL);
        let mut shader_defs = Vec::new();
        if subpixel {
            shader_defs.push("SUBPIXEL".into());
        }
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
            } else if method == SpritePipelineKey::TONEMAP_METHOD_PBR_NEUTRAL {
                shader_defs.push("TONEMAP_METHOD_PBR_NEUTRAL".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(SpritePipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if key.contains(SpritePipelineKey::SRGB_COMPOSITING) {
            shader_defs.push("SRGB_OUTPUT".into());
        }
        if key.contains(SpritePipelineKey::OKLAB_COMPOSITING) {
            shader_defs.push("OKLAB_OUTPUT".into());
        }

        let format = key.target_format();

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

        // Subpixel text renders via a dual-source-blend fragment shader so
        // the framebuffer can consume per-channel alpha from `@blend_src(1)`.
        // Color blend:
        //   result.rgb = fg.rgb * alpha_per_channel
        //              + dst.rgb * (1 - alpha_per_channel)
        // which requires `Src1` / `OneMinusSrc1` (the per-channel alpha is
        // the dual-source output). Alpha keeps conventional premultiplied
        // behaviour so the framebuffer's own alpha stays sensible. Mirrors
        // `bevy_ui_render::UiPipeline::specialize`'s subpixel branch
        // (phase-04 of spec/0013) and GPUI's subpixel pipeline.
        let (fragment_entry_point, blend) = if subpixel {
            (
                Some("fragment_subpixel".into()),
                BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Src1,
                        dst_factor: BlendFactor::OneMinusSrc1,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                },
            )
        } else {
            (None, BlendState::ALPHA_BLENDING)
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
                entry_point: fragment_entry_point,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(blend),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.material_layout.clone()],
            // Sprites are always alpha blended so they never need to write to depth.
            // They just need to read it in case an opaque mesh2d
            // that wrote to depth is present.
            depth_stencil: Some(DepthStencilState {
                format: CORE_2D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
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
            label: Some(if subpixel {
                "sprite_pipeline_subpixel".into()
            } else {
                "sprite_pipeline".into()
            }),
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
    /// Whether this sprite renders through the RGB subpixel antialiased
    /// text pipeline variant (`fragment_subpixel` in `sprite.wgsl`,
    /// dual-source blend). Only set by [`crate::extract_text2d_sprite`] for
    /// glyphs whose atlas was produced with
    /// [`FontSmoothing::SubpixelAntiAliased`](bevy_text::FontSmoothing::SubpixelAntiAliased)
    /// and when the adapter advertises
    /// [`wgpu::Features::DUAL_SOURCE_BLENDING`](https://docs.rs/wgpu/latest/wgpu/struct.Features.html#associatedconstant.DUAL_SOURCE_BLENDING)
    /// (see [`bevy_text::SubpixelCapable`]).
    ///
    /// Defaults to `false` for all non-text sprites and for grayscale
    /// glyph batches — those use the standard alpha-blend path.
    pub subpixel: bool,
}

pub enum ExtractedSpriteKind {
    /// A single sprite with custom sizing and scaling options
    Single {
        anchor: Vec2,
        rect: Option<Rect>,
        scaling_mode: Option<SpriteScalingMode>,
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
                subpixel: false,
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
                subpixel: false,
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
    /// Uniform buffer for [`SubpixelTextUniforms`]. Bound at `@group(0)
    /// @binding(3)` alongside the view uniform for *all* sprite pipeline
    /// variants — the non-subpixel fragment entry ignores it, but keeping
    /// the bind-group layout stable avoids a separate `view_layout` per
    /// variant. Seeded with [`SubpixelTextUniforms::default`];
    /// [`prepare_sprite_view_bind_groups`] overwrites the value each frame
    /// from the main-world [`bevy_text::SubpixelTextSettings`] /
    /// [`bevy_text::SubpixelLcdLayout`] resources when the `bevy_text`
    /// feature is enabled.
    pub(crate) subpixel_settings: UniformBuffer<SubpixelTextUniforms>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            sprite_index_buffer: RawBufferVec::<u32>::new(BufferUsages::INDEX),
            sprite_instance_buffer: RawBufferVec::<SpriteInstance>::new(BufferUsages::VERTEX),
            subpixel_settings: UniformBuffer::from(SubpixelTextUniforms::default()),
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
    mut cameras: Query<(
        &RenderVisibleEntities,
        &ExtractedCamera,
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
) {
    let draw_sprite_function = draw_functions.read().id::<DrawSprite>();

    for (visible_entities, camera, view, msaa, tonemapping, dither) in &mut cameras {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let msaa_key = SpritePipelineKey::from_msaa_samples(msaa.samples());
        let mut view_key = SpritePipelineKey::from_target_format(view.target_format) | msaa_key;

        if camera
            .compositing_space
            .is_some_and(|s| s == bevy_camera::CompositingSpace::Srgb)
        {
            view_key |= SpritePipelineKey::SRGB_COMPOSITING;
        }
        if camera
            .compositing_space
            .is_some_and(|s| s == bevy_camera::CompositingSpace::Oklab)
        {
            view_key |= SpritePipelineKey::OKLAB_COMPOSITING;
        }

        if !camera.hdr {
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
                    Tonemapping::PbrNeutral => SpritePipelineKey::TONEMAP_METHOD_PBR_NEUTRAL,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= SpritePipelineKey::DEBAND_DITHER;
            }
        }

        // Base pipeline variant for non-subpixel sprites. The subpixel
        // variant is specialised on-demand below per-sprite so pure
        // grayscale workloads don't pay for a second pipeline compile.
        let pipeline = pipelines.specialize(&pipeline_cache, &sprite_pipeline, view_key);

        view_entities.clear();
        if let Some(visible_entities) = visible_entities.get::<Sprite>() {
            view_entities.extend(
                visible_entities
                    .iter_visible()
                    .map(|(_, e)| e.index_u32() as usize),
            );
        }

        transparent_phase
            .items
            .reserve(extracted_sprites.sprites.len());

        for (index, extracted_sprite) in extracted_sprites.sprites.iter().enumerate() {
            let view_index = extracted_sprite.main_entity.index_u32();

            if !view_entities.contains(view_index as usize) {
                continue;
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_sprite.transform.translation().z);

            // Mirrors the pattern in `bevy_ui_render::queue_uinodes`: the
            // subpixel pipeline variant is selected per extracted sprite
            // (not per view) because a single view can mix subpixel text
            // sprites with regular sprites, and each needs its own pipeline.
            let item_pipeline = if extracted_sprite.subpixel {
                pipelines.specialize(
                    &pipeline_cache,
                    &sprite_pipeline,
                    view_key | SpritePipelineKey::SUBPIXEL,
                )
            } else {
                pipeline
            };

            // Add the item to the render phase
            transparent_phase.add_transient(Transparent2d {
                draw_function: draw_sprite_function,
                pipeline: item_pipeline,
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
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    sprite_pipeline: Res<SpritePipeline>,
    view_uniforms: Res<ViewUniforms>,
    mut sprite_meta: ResMut<SpriteMeta>,
    views: Query<(Entity, &Tonemapping), With<ExtractedView>>,
    tonemapping_luts: Res<TonemappingLuts>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
    #[cfg(feature = "bevy_text")] subpixel_settings: Option<Res<SubpixelTextSettings>>,
    #[cfg(feature = "bevy_text")] subpixel_layout: Option<Res<SubpixelLcdLayout>>,
) {
    // Drive the subpixel-text uniform from the main-world tuning resources.
    // `TextPlugin::build` `init_resource`'s both with GPUI-derived defaults;
    // a missing resource (headless / no `TextPlugin` / `bevy_text` feature
    // disabled) falls back to [`SubpixelTextUniforms::default`] so the bind
    // group still populates correctly.
    #[cfg(feature = "bevy_text")]
    {
        let settings = subpixel_settings.as_deref().copied().unwrap_or_default();
        let layout = subpixel_layout.as_deref().copied().unwrap_or_default();
        sprite_meta.subpixel_settings.set(SubpixelTextUniforms {
            enhanced_contrast: settings.enhanced_contrast,
            layout_flags: layout.pack_u32(),
            _pad: Vec2::ZERO,
            gamma_ratios: settings.gamma_ratios,
        });
    }

    // Flush the subpixel-text uniform to the GPU before building the view
    // bind group below — `subpixel_settings.binding()` returns `None` until
    // the backing buffer has been written at least once. Mirrors the
    // sequence in `bevy_ui_render::prepare_uinodes`.
    sprite_meta
        .subpixel_settings
        .write_buffer(&render_device, &render_queue);

    let (Some(view_binding), Some(subpixel_binding)) = (
        view_uniforms.uniforms.binding(),
        sprite_meta.subpixel_settings.binding(),
    ) else {
        return;
    };

    for (entity, tonemapping) in &views {
        let lut_bindings =
            get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
        let view_bind_group = render_device.create_bind_group(
            "mesh2d_view_bind_group",
            &pipeline_cache.get_bind_group_layout(&sprite_pipeline.view_layout),
            &BindGroupEntries::sequential((
                view_binding.clone(),
                lut_bindings.0,
                lut_bindings.1,
                subpixel_binding.clone(),
            )),
        );

        commands.entity(entity).insert(SpriteViewBindGroup {
            value: view_bind_group,
        });
    }
}

pub fn prepare_sprite_image_bind_groups(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
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
        let mut batch_image_handle = None;

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
                batch_image_handle = None;
                continue;
            };

            if batch_image_handle != Some(extracted_sprite.image_handle_id) {
                let Some(gpu_image) = gpu_images.get(extracted_sprite.image_handle_id) else {
                    continue;
                };

                batch_image_size = gpu_image.size_2d().as_vec2();
                let image_handle = extracted_sprite.image_handle_id;
                batch_image_handle = Some(image_handle);
                image_bind_groups
                    .values
                    .entry(image_handle)
                    .or_insert_with(|| {
                        render_device.create_bind_group(
                            "sprite_material_bind_group",
                            &pipeline_cache.get_bind_group_layout(&sprite_pipeline.material_layout),
                            &BindGroupEntries::sequential((
                                &gpu_image.texture_view,
                                &gpu_image.sampler,
                            )),
                        )
                    });

                batch_item_index = item_index;
                current_batch = Some(batches.entry((*retained_view, item.entity())).insert(
                    SpriteBatch {
                        image_handle_id: image_handle,
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
    scaling_mode: SpriteScalingMode,
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
        SpriteScalingMode::FillCenter => {
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
        SpriteScalingMode::FillStart => {
            if quad_ratio > texture_ratio {
                uv_offset_scale.y += uv_offset_scale.w - uv_offset_scale.w * tex_quad_scale;
                uv_offset_scale.w *= tex_quad_scale;
            } else {
                uv_offset_scale.z *= quad_tex_scale;
            }
        }
        SpriteScalingMode::FillEnd => {
            if quad_ratio > texture_ratio {
                uv_offset_scale.w *= tex_quad_scale;
            } else {
                uv_offset_scale.x += uv_offset_scale.z - uv_offset_scale.z * quad_tex_scale;
                uv_offset_scale.z *= quad_tex_scale;
            }
        }
        SpriteScalingMode::FitCenter => {
            if texture_ratio > quad_ratio {
                // Scale based on width
                quad_size.y *= quad_tex_scale;
            } else {
                // Scale based on height
                quad_size.x *= tex_quad_scale;
            }
        }
        SpriteScalingMode::FitStart => {
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
        SpriteScalingMode::FitEnd => {
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
