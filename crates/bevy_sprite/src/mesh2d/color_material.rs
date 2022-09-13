use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_math::Vec4;
use bevy_reflect::TypeUuid;
use bevy_render::{
    color::Color, prelude::Shader, render_asset::RenderAssets, render_resource::*, texture::Image,
};

use crate::{Material2d, Material2dPlugin, MaterialMesh2dBundle};

pub const COLOR_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3253086872234592509);

#[derive(Default)]
pub struct ColorMaterialPlugin;

impl Plugin for ColorMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            COLOR_MATERIAL_SHADER_HANDLE,
            "color_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugin(Material2dPlugin::<ColorMaterial>::default());

        app.world
            .resource_mut::<Assets<ColorMaterial>>()
            .set_untracked(
                Handle::<ColorMaterial>::default(),
                ColorMaterial {
                    color: Color::rgb(1.0, 0.0, 1.0),
                    ..Default::default()
                },
            );
    }
}

/// A [2d material](Material2d) that renders [2d meshes](crate::Mesh2dHandle) with a texture tinted by a uniform color
#[derive(AsBindGroup, Debug, Clone, TypeUuid)]
#[uuid = "e228a544-e3ca-4e1e-bb9d-4d8bc1ad8c19"]
#[uniform(0, ColorMaterialUniform)]
pub struct ColorMaterial {
    pub color: Color,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl Default for ColorMaterial {
    fn default() -> Self {
        ColorMaterial {
            color: Color::WHITE,
            texture: None,
        }
    }
}

impl From<Color> for ColorMaterial {
    fn from(color: Color) -> Self {
        ColorMaterial {
            color,
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

// NOTE: These must match the bit flags in bevy_sprite/src/mesh2d/color_material.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ColorMaterialFlags: u32 {
        const TEXTURE           = (1 << 0);
        const NONE              = 0;
        const UNINITIALIZED     = 0xFFFF;
    }
}

/// The GPU representation of the uniform data of a [`ColorMaterial`].
#[derive(Clone, Default, ShaderType)]
pub struct ColorMaterialUniform {
    pub color: Vec4,
    pub flags: u32,
}

impl AsBindGroupShaderType<ColorMaterialUniform> for ColorMaterial {
    fn as_bind_group_shader_type(&self, _images: &RenderAssets<Image>) -> ColorMaterialUniform {
        let mut flags = ColorMaterialFlags::NONE;
        if self.texture.is_some() {
            flags |= ColorMaterialFlags::TEXTURE;
        }

        ColorMaterialUniform {
            color: self.color.as_linear_rgba_f32().into(),
            flags: flags.bits(),
        }
    }
}

impl Material2d for ColorMaterial {
    fn fragment_shader() -> ShaderRef {
        COLOR_MATERIAL_SHADER_HANDLE.typed().into()
    }
}

/// A component bundle for entities with a [`Mesh2dHandle`](crate::Mesh2dHandle) and a [`ColorMaterial`].
pub type ColorMesh2dBundle = MaterialMesh2dBundle<ColorMaterial>;
