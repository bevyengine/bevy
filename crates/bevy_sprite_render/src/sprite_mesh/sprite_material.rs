use bevy_app::Plugin;
use bevy_color::{Color, ColorToComponents};
use bevy_image::Image;
use bevy_math::{Affine2, Mat3, Vec2, Vec4};

use bevy_asset::{embedded_asset, embedded_path, Asset, AssetApp, AssetPath, Handle};

use bevy_reflect::Reflect;
use bevy_render::{
    render_asset::RenderAssets,
    render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderType},
};
use bevy_shader::ShaderRef;
use bevy_sprite::{prelude::SpriteMesh, SpriteAlphaMode};

use crate::{AlphaMode2d, Material2d, Material2dPlugin};

pub struct SpriteMaterialPlugin;

impl Plugin for SpriteMaterialPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        embedded_asset!(app, "sprite_material.wgsl");

        app.add_plugins(Material2dPlugin::<SpriteMaterial>::default())
            .register_asset_reflect::<SpriteMaterial>();
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Debug, Clone)]
#[uniform(0, SpriteMaterialUniform)]
pub struct SpriteMaterial {
    pub color: Color,
    pub alpha_mode: AlphaMode2d,
    pub uv_transform: Affine2,
    pub scale: Vec2,
    #[texture(1)]
    #[sampler(2)]
    pub image: Handle<Image>,
}

// NOTE: These must match the bit flags in bevy_sprite_render/src/sprite_mesh/sprite_materials.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct SpriteMaterialFlags: u32 {
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

#[derive(ShaderType)]
pub struct SpriteMaterialUniform {
    pub color: Vec4,
    pub uv_transform: Mat3,
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub scale: Vec2,
}

impl AsBindGroupShaderType<SpriteMaterialUniform> for SpriteMaterial {
    fn as_bind_group_shader_type(
        &self,
        _images: &RenderAssets<bevy_render::texture::GpuImage>,
    ) -> SpriteMaterialUniform {
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

        SpriteMaterialUniform {
            color: self.color.to_linear().to_vec4(),
            uv_transform: self.uv_transform.into(),
            flags: flags.bits(),
            alpha_cutoff,
            scale: self.scale,
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

impl From<SpriteMesh> for SpriteMaterial {
    fn from(value: SpriteMesh) -> Self {
        // convert SpriteAlphaMode to AlphaMode2d (see the comment above SpriteAlphaMode)
        let alpha_mode = match value.alpha_mode {
            SpriteAlphaMode::Blend => AlphaMode2d::Blend,
            SpriteAlphaMode::Opaque => AlphaMode2d::Opaque,
            SpriteAlphaMode::Mask(x) => AlphaMode2d::Mask(x),
        };

        SpriteMaterial {
            color: value.color,
            uv_transform: Affine2::default(),
            image: value.image,
            alpha_mode,
            scale: Vec2::default(),
        }
    }
}
