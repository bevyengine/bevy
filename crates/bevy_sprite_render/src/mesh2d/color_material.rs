use crate::{AlphaMode2d, Material2d, Material2dPlugin};
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, embedded_path, Asset, AssetApp, AssetPath, Assets, Handle};
use bevy_color::{Alpha, Color, ColorToComponents, LinearRgba};
use bevy_image::Image;
use bevy_math::{Affine2, Mat3, Vec4};
use bevy_reflect::prelude::*;
use bevy_render::{render_asset::RenderAssets, render_resource::*, texture::GpuImage};
use bevy_shader::ShaderRef;

#[derive(Default)]
pub struct ColorMaterialPlugin;

impl Plugin for ColorMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "color_material.wgsl");

        app.add_plugins(Material2dPlugin::<ColorMaterial>::default())
            .register_asset_reflect::<ColorMaterial>();

        // Initialize the default material handle.
        app.world_mut()
            .resource_mut::<Assets<ColorMaterial>>()
            .insert(
                &Handle::<ColorMaterial>::default(),
                ColorMaterial {
                    color: Color::srgb(1.0, 0.0, 1.0),
                    ..Default::default()
                },
            )
            .unwrap();
    }
}

/// A [2d material](Material2d) that renders [2d meshes](crate::Mesh2d) with a texture tinted by a uniform color
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Default, Debug, Clone)]
#[uniform(0, ColorMaterialUniform)]
pub struct ColorMaterial {
    pub color: Color,
    pub alpha_mode: AlphaMode2d,
    pub uv_transform: Affine2,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl ColorMaterial {
    /// Creates a new material from a given color
    pub fn from_color(color: impl Into<Color>) -> Self {
        Self::from(color.into())
    }
}

impl Default for ColorMaterial {
    fn default() -> Self {
        ColorMaterial {
            color: Color::WHITE,
            uv_transform: Affine2::default(),
            texture: None,
            // TODO should probably default to AlphaMask once supported?
            alpha_mode: AlphaMode2d::Blend,
        }
    }
}

impl From<Color> for ColorMaterial {
    fn from(color: Color) -> Self {
        ColorMaterial {
            color,
            alpha_mode: if color.alpha() < 1.0 {
                AlphaMode2d::Blend
            } else {
                AlphaMode2d::Opaque
            },
            ..Default::default()
        }
    }
}

impl From<Handle<Image>> for ColorMaterial {
    fn from(texture: Handle<Image>) -> Self {
        ColorMaterial {
            texture: Some(texture),
            ..Default::default()
        }
    }
}

// NOTE: These must match the bit flags in bevy_sprite_render/src/mesh2d/color_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ColorMaterialFlags: u32 {
        const TEXTURE                    = 1 << 0;
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

impl ColorMaterialFlags {
    const ALPHA_MODE_MASK_BITS: u32 = 0b11;
    const ALPHA_MODE_SHIFT_BITS: u32 = 32 - Self::ALPHA_MODE_MASK_BITS.count_ones();
}

/// The GPU representation of the uniform data of a [`ColorMaterial`].
#[derive(Clone, Default, ShaderType)]
pub struct ColorMaterialUniform {
    pub color: Vec4,
    pub uv_transform: Mat3,
    pub flags: u32,
    pub alpha_cutoff: f32,
}

impl AsBindGroupShaderType<ColorMaterialUniform> for ColorMaterial {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<GpuImage>) -> ColorMaterialUniform {
        let mut flags = ColorMaterialFlags::NONE;
        if self.texture.is_some() {
            flags |= ColorMaterialFlags::TEXTURE;
        }

        // Defaults to 0.5 like in 3d
        let mut alpha_cutoff = 0.5;
        match self.alpha_mode {
            AlphaMode2d::Opaque => flags |= ColorMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode2d::Mask(c) => {
                alpha_cutoff = c;
                flags |= ColorMaterialFlags::ALPHA_MODE_MASK;
            }
            AlphaMode2d::Blend => flags |= ColorMaterialFlags::ALPHA_MODE_BLEND,
        };
        ColorMaterialUniform {
            color: LinearRgba::from(self.color).to_f32_array().into(),
            uv_transform: self.uv_transform.into(),
            flags: flags.bits(),
            alpha_cutoff,
        }
    }
}

impl Material2d for ColorMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("color_material.wgsl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }
}
