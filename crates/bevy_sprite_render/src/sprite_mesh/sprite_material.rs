use std::f32::consts::PI;

use bevy_app::Plugin;
use bevy_color::{Color, ColorToComponents};
use bevy_image::{Image, TextureAtlas};
use bevy_math::{vec2, vec3, Affine2, Affine3, Affine3A, Mat3, Rect, Vec2, Vec4};

use bevy_asset::{embedded_asset, embedded_path, Asset, AssetApp, AssetPath, Handle};

use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_reflect::Reflect;
use bevy_render::{
    render_asset::RenderAssets,
    render_resource::{
        binding_types::sampler, AsBindGroup, AsBindGroupShaderType, BindGroupLayoutDescriptor,
        RenderPipelineDescriptor, SamplerBindingType, ShaderType, SpecializedMeshPipelineError,
    },
};
use bevy_shader::{ShaderDefVal, ShaderRef};
use bevy_sprite::{prelude::SpriteMesh, SpriteAlphaMode, SpriteImageMode};

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

#[derive(ShaderType, Default)]
pub struct SpriteMaterialUniform {
    pub color: Vec4,
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub scale: Vec2,
    pub uv_transform: Mat3,
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

        let image_size = image.size_2d().as_vec2();

        let mut scale = image_size;
        let mut affine = Affine2::default();

        if let Some(rect) = self.rect {
            let ratio = rect.size() / image_size;

            affine *= Affine2::from_scale(ratio);
            affine *= Affine2::from_translation(vec2(
                rect.min.x / rect.size().x,
                rect.min.y / rect.size().y,
            ));

            scale = rect.size();
        }

        if let Some(custom_size) = self.custom_size {
            scale = custom_size;
        }

        SpriteMaterialUniform {
            color: self.color.to_linear().to_vec4(),
            flags: flags.bits(),
            alpha_cutoff,
            scale,
            uv_transform: affine.into(),
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

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: crate::Material2dKey<Self>,
    ) -> bevy_ecs::error::Result<(), SpecializedMeshPipelineError> {
        // descriptor.fragment.unwrap().shader_defs.push(ShaderDefVal::);
        Ok(())
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
        }
    }
}
