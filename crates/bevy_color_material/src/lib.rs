//! Simple color material for 2d meshes

mod material;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, AssetApp, Assets, Handle};
use bevy_color::Color;
use bevy_render::render_resource::Shader;
use bevy_render_2d::Material2dPlugin;

pub use material::ColorMaterial;

#[doc(hidden)]
pub mod prelude {
    pub use super::material::ColorMaterial;
}

const COLOR_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("92e0e6e9-ed0b-4db3-89ab-5f65d3678250");

/// Plugin that sets up [`ColorMaterial`] related assets, systems, and shaders
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
            );
    }
}
