//! Demonstrates using a custom extension to the `StandardMaterial` to create a repeating texture that avoids seams
//! by using stochastic sampling. This example uses a custom shader to achieve the effect.
use bevy::image::{ImageAddressMode, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                address_mode_w: ImageAddressMode::Repeat,
                ..Default::default()
            },
        }))
        .add_plugins(Material2dPlugin::<CustomMaterial>::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera2d);
    let texture = asset_server.load("textures/rocks.png");
    commands.spawn((
        Mesh2d(meshes.add(repeating_quad(10.0))),
        MeshMaterial2d(materials.add(CustomMaterial {
            texture: Some(texture),
        })),
        Transform::default(),
    ));
}

// This struct defines the data that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[texture(1)]
    #[sampler(2)]
    texture: Option<Handle<Image>>,
}

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/stochastic_sampling.wgsl";

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

/// Creates a quad where the texture repeats n times in both directions.
fn repeating_quad(n: f32) -> Mesh {
    let mut mesh: Mesh = Rectangle::from_length(1000.0).into();
    let uv_attribute = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0).unwrap();
    // The format of the UV coordinates should be Float32x2.
    let VertexAttributeValues::Float32x2(uv_attribute) = uv_attribute else {
        panic!("Unexpected vertex format, expected Float32x2.");
    };
    // The default `Rectangle`'s texture coordinates are in the range of `0..=1`. Values outside
    // this range cause the texture to repeat.
    for uv_coord in uv_attribute.iter_mut() {
        uv_coord[0] *= n;
        uv_coord[1] *= n;
    }
    mesh
}
