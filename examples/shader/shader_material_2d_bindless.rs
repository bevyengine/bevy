//! Demonstrates bindless materials in 2D.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

/// This example uses a shader source file from the `assets` subdirectory.
const SHADER_ASSET_PATH: &str = "shaders/custom_material_2d_bindless.wgsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            Material2dPlugin::<BindlessMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

// Setup a simple 2D scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Spawn the camera.
    commands.spawn(Camera2d);

    // Create the shared assets that the bindless material will use.
    let dark_icon = asset_server.load("branding/bevy_logo_dark.png");
    let light_icon = asset_server.load("branding/bevy_logo_light.png");
    let mesh = meshes.add(Rectangle::default());

    // Spawn the quads.
    for (offset, color, image) in [
        (vec2(-1.0, -1.0), LinearRgba::RED, dark_icon.clone()),
        (
            vec2(1.0, -1.0),
            LinearRgba::new(1.0, 1.0, 0.0, 1.0),
            light_icon.clone(),
        ),
        (vec2(-1.0, 1.0), LinearRgba::GREEN, dark_icon.clone()),
        (vec2(1.0, 1.0), LinearRgba::BLUE, light_icon.clone()),
    ] {
        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(BindlessMaterial {
                color,
                color_texture: Some(image),
            })),
            Transform::default()
                .with_scale(vec3(128.0, 32.0, 1.0))
                .with_translation((offset * 128.0).extend(0.0)),
        ));
    }
}

/// The CPU-side structure that's passed to the shader.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[uniform(0, BindlessMaterialUniform, binding_array(10))]
#[bindless]
struct BindlessMaterial {
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
}

/// The GPU-side structure that's passed to the shader.
#[derive(ShaderType)]
struct BindlessMaterialUniform {
    color: LinearRgba,
}

// The conversion function that Bevy calls to convert from the CPU-side material
// structure to the GPU-side material structure.
impl<'a> From<&'a BindlessMaterial> for BindlessMaterialUniform {
    fn from(material: &'a BindlessMaterial) -> Self {
        BindlessMaterialUniform {
            color: material.color,
        }
    }
}

// The `Material2d`` trait is very configurable, but comes with sensible defaults
// for all methods.
//
// You only need to implement functions for features that need non-default
// behavior. See the `Material2d` API documentation for details.
impl Material2d for BindlessMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Mask(0.5)
    }
}
