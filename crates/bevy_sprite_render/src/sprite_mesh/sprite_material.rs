use core::f32;

use bevy_app::Plugin;
use bevy_color::{Color, ColorToComponents};
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout};
use bevy_math::{vec2, Affine2, Mat3, Rect, Vec2, Vec4};

use bevy_asset::{embedded_asset, embedded_path, Asset, AssetApp, AssetPath, Handle};

use bevy_reflect::Reflect;
use bevy_render::{
    render_asset::RenderAssets,
    render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderType},
};
use bevy_shader::ShaderRef;
use bevy_sprite::{
    prelude::SpriteMesh, SliceScaleMode, SpriteAlphaMode, SpriteImageMode, SpriteScalingMode,
};

use crate::{AlphaMode2d, Material2d, Material2dPlugin};

pub struct SpriteMaterialPlugin;

impl Plugin for SpriteMaterialPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        embedded_asset!(app, "sprite_material.wgsl");

        app.add_plugins(Material2dPlugin::<SpriteMaterial>::default())
            .register_asset_reflect::<SpriteMaterial>();
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
#[reflect(Debug, Clone)]
#[uniform(0, SpriteMaterialUniform)]
pub struct SpriteMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub image: Handle<Image>,
    pub texture_atlas: Option<TextureAtlas>,
    pub color: Color,
    pub flip_x: bool,
    pub flip_y: bool,
    pub custom_size: Option<Vec2>,
    pub rect: Option<Rect>,
    pub image_mode: SpriteImageMode,
    pub alpha_mode: AlphaMode2d,
    pub anchor: Vec2,
    pub texture_atlas_layout: Option<TextureAtlasLayout>,
    pub texture_atlas_index: usize,
}

// NOTE: These must match the bit flags in bevy_sprite_render/src/sprite_mesh/sprite_materials.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct SpriteMaterialFlags: u32 {
        const FLIP_X = 1;
        const FLIP_Y = 2;
        const TILE_X = 4;
        const TILE_Y = 8;
        /// Bitmask reserving bits for the [`AlphaMode2d`]
        /// Values are just sequential values bitshifted into
        /// the bitmask, and can range from 0 to 3.
        const ALPHA_MODE_RESERVED_BITS   = Self::ALPHA_MODE_MASK_BITS << Self::ALPHA_MODE_SHIFT_BITS;
        const ALPHA_MODE_OPAQUE          = 0 << Self::ALPHA_MODE_SHIFT_BITS;
        const ALPHA_MODE_MASK            = 1 << Self::ALPHA_MODE_SHIFT_BITS;
        const ALPHA_MODE_BLEND           = 2 << Self::ALPHA_MODE_SHIFT_BITS;
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

impl SpriteMaterialFlags {
    const ALPHA_MODE_MASK_BITS: u32 = 0b11;
    const ALPHA_MODE_SHIFT_BITS: u32 = 32 - Self::ALPHA_MODE_MASK_BITS.count_ones();
}

#[derive(ShaderType, Default)]
pub struct SpriteMaterialUniform {
    pub color: Vec4,
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub vertex_scale: Vec2,
    pub vertex_offset: Vec2,
    pub uv_transform: Mat3,

    // tile shader def
    pub tile_stretch_value: Vec2,

    // slice shader def
    pub scale: Vec2,
    pub min_inset: Vec2,
    pub max_inset: Vec2,
    pub side_stretch_value: Vec2,
    pub center_stretch_value: Vec2,
}

#[derive(ShaderType, Default)]
pub struct SpriteMaterialTile {
    pub stretch_value: f32,
}

impl AsBindGroupShaderType<SpriteMaterialUniform> for SpriteMaterial {
    fn as_bind_group_shader_type(
        &self,
        images: &RenderAssets<bevy_render::texture::GpuImage>,
    ) -> SpriteMaterialUniform {
        let Some(image) = images.get(self.image.id()) else {
            return SpriteMaterialUniform::default();
        };

        let mut flags = SpriteMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        match self.alpha_mode {
            AlphaMode2d::Opaque => flags |= SpriteMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode2d::Mask(c) => {
                alpha_cutoff = c;
                flags |= SpriteMaterialFlags::ALPHA_MODE_MASK;
            }
            AlphaMode2d::Blend => flags |= SpriteMaterialFlags::ALPHA_MODE_BLEND,
        };

        if self.flip_x {
            flags |= SpriteMaterialFlags::FLIP_X;
        }
        if self.flip_y {
            flags |= SpriteMaterialFlags::FLIP_Y;
        }

        let mut image_size = image.size_2d().as_vec2();

        let mut quad_size = image_size;
        let mut quad_offset = Vec2::ZERO;
        let mut uv_transform = Affine2::default();

        if let Some(texture_atlas_layout) = &self.texture_atlas_layout {
            let index = self
                .texture_atlas_index
                .clamp(0, texture_atlas_layout.textures.len() - 1);

            let rect = texture_atlas_layout.textures[index].as_rect();

            let ratio = rect.size() / image_size;

            uv_transform *= Affine2::from_scale(ratio);
            uv_transform *= Affine2::from_translation(vec2(
                rect.min.x as f32 / rect.size().y as f32,
                rect.min.y as f32 / rect.size().y as f32,
            ));

            quad_size = rect.size();
            image_size = rect.size();
        }

        // rect selects a slice of the image to render, map the uv and change the quad scale to match the rect
        if let Some(rect) = self.rect {
            let ratio = rect.size() / image_size;

            uv_transform *= Affine2::from_scale(ratio);
            uv_transform *= Affine2::from_translation(vec2(
                rect.min.x / rect.size().x,
                rect.min.y / rect.size().y,
            ));

            quad_size = rect.size();
            image_size = rect.size();
        }

        let mut tile_stretch_value = Vec2::ZERO;

        let mut scale = Vec2::ZERO;
        let mut min_inset = Vec2::ZERO;
        let mut max_inset = Vec2::ZERO;
        let mut side_stretch_value = Vec2::ZERO;
        let mut center_stretch_value = Vec2::ZERO;

        if let Some(custom_size) = self.custom_size {
            match &self.image_mode {
                SpriteImageMode::Auto => {
                    quad_size = custom_size;
                }
                SpriteImageMode::Scale(scaling_mode) => {
                    let quad_ratio = quad_size.x / quad_size.y;
                    let custom_ratio = custom_size.x / custom_size.y;

                    let fill_size = || {
                        if quad_ratio > custom_ratio {
                            vec2(custom_size.y * quad_ratio, custom_size.y)
                        } else {
                            vec2(custom_size.x, custom_size.x / quad_ratio)
                        }
                    };

                    let fit_size = || {
                        if quad_ratio > custom_ratio {
                            vec2(custom_size.x, custom_size.x / quad_ratio)
                        } else {
                            vec2(custom_size.y * quad_ratio, custom_size.y)
                        }
                    };

                    match scaling_mode {
                        // Filling requires scaling the texture and cutting out the 'overflow'
                        // which is why we need to manipulate the UV.
                        SpriteScalingMode::FillCenter => {
                            let fill_size = fill_size();
                            uv_transform *= Affine2::from_scale(custom_size / fill_size);
                            uv_transform *= Affine2::from_translation(
                                (fill_size - custom_size) * 0.5 / custom_size,
                            );
                            quad_size = custom_size;
                        }
                        SpriteScalingMode::FillStart => {
                            let fill_size = fill_size();
                            uv_transform *= Affine2::from_scale(custom_size / fill_size);
                            quad_size = custom_size;
                        }
                        SpriteScalingMode::FillEnd => {
                            let fill_size = fill_size();
                            uv_transform *= Affine2::from_scale(custom_size / fill_size);
                            uv_transform *=
                                Affine2::from_translation((fill_size - custom_size) / custom_size);
                            quad_size = custom_size;
                        }

                        // Fitting is easier since the whole texture will still be visible,
                        // so it's enough to just translate the quad and keep the UV as is.
                        SpriteScalingMode::FitCenter => {
                            let fit_size = fit_size();
                            quad_size = fit_size;
                        }
                        SpriteScalingMode::FitStart => {
                            let fit_size = fit_size();
                            quad_offset -= (custom_size - fit_size) * 0.5;
                            quad_size = fit_size;
                        }
                        SpriteScalingMode::FitEnd => {
                            let fit_size = fit_size();
                            quad_offset += (custom_size - fit_size) * 0.5;
                            quad_size = fit_size;
                        }
                    }
                }
                SpriteImageMode::Tiled {
                    tile_x,
                    tile_y,
                    stretch_value,
                } => {
                    if *tile_x {
                        flags |= SpriteMaterialFlags::TILE_X;
                    }
                    if *tile_y {
                        flags |= SpriteMaterialFlags::TILE_Y;
                    }

                    // This is the [0-1] x and y of where the UV should start repeating.
                    // E.g. if the stretch_value x is 0.2 and the UV x is 0.5, it will be mapped to 0.1 (0.5 - 0.2 * 2)
                    // and then be stretched over [0, 0.2] by translating it to (0.1 / 0.2) = 0.5,
                    // so it corresponds to the center of the texture.
                    tile_stretch_value = (image_size * stretch_value) / custom_size;
                    quad_size = custom_size;
                }
                SpriteImageMode::Sliced(slicer) => {
                    let quad_ratio = quad_size.x / quad_size.y;
                    let custom_ratio = custom_size.x / custom_size.y;

                    if quad_ratio > custom_ratio {
                        scale = vec2(1.0, quad_ratio / custom_ratio)
                    } else {
                        scale = vec2(custom_ratio / quad_ratio, 1.0)
                    }

                    min_inset = slicer.border.min_inset / quad_size;
                    max_inset = slicer.border.max_inset / quad_size;

                    let corner_scale = slicer.max_corner_scale.clamp(f32::EPSILON, 1.0);
                    scale /= corner_scale;

                    if let SliceScaleMode::Tile { stretch_value } = slicer.sides_scale_mode {
                        side_stretch_value = stretch_value
                            * (image_size * (1.0 - max_inset - min_inset))
                            / (custom_size * (1.0 - max_inset / scale - min_inset / scale));
                    }

                    if let SliceScaleMode::Tile { stretch_value } = slicer.center_scale_mode {
                        center_stretch_value = stretch_value
                            * (image_size * (1.0 - max_inset - min_inset))
                            / (custom_size * (1.0 - max_inset / scale - min_inset / scale));
                    }

                    quad_size = custom_size;
                }
            }
        }

        SpriteMaterialUniform {
            color: self.color.to_linear().to_vec4(),
            flags: flags.bits(),
            alpha_cutoff,
            vertex_scale: quad_size,
            vertex_offset: quad_offset,
            uv_transform: uv_transform.into(),

            tile_stretch_value,

            scale,
            min_inset,
            max_inset,
            side_stretch_value,
            center_stretch_value,
        }
    }
}

impl Material2d for SpriteMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("sprite_material.wgsl"))
                .with_source("embedded"),
        )
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("sprite_material.wgsl"))
                .with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }
}

impl SpriteMaterial {
    /// Use the [`SpriteMesh`] to build a new material.
    pub fn from_sprite_mesh(sprite: SpriteMesh) -> Self {
        // convert SpriteAlphaMode to AlphaMode2d.
        // (see the comment above SpriteAlphaMode for why these are different)
        let alpha_mode = match sprite.alpha_mode {
            SpriteAlphaMode::Blend => AlphaMode2d::Blend,
            SpriteAlphaMode::Opaque => AlphaMode2d::Opaque,
            SpriteAlphaMode::Mask(x) => AlphaMode2d::Mask(x),
        };

        SpriteMaterial {
            image: sprite.image,
            texture_atlas: sprite.texture_atlas,
            color: sprite.color,
            flip_x: sprite.flip_x,
            flip_y: sprite.flip_y,
            custom_size: sprite.custom_size,
            rect: sprite.rect,
            image_mode: sprite.image_mode,
            alpha_mode,
            texture_atlas_layout: None,
            texture_atlas_index: 0,
            anchor: Vec2::ZERO,
        }
    }
}
