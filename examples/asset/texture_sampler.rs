//! By default Bevy loads images to textures with sampler settings that clamps the image to the edges
//! (UV coordinates outside of the range `0..=1` are clamped to `0..=1`).
//! This example shows how to change the sampler settings to repeat the image instead.

use bevy::app::App;
use bevy::app::Startup;
use bevy::asset::AssetServer;
use bevy::asset::Assets;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::render::mesh::Indices;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::texture::ImageAddressMode;
use bevy::render::texture::ImageLoaderSettings;
use bevy::render::texture::ImageSampler;
use bevy::render::texture::ImageSamplerDescriptor;
use bevy::sprite::MaterialMesh2dBundle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Quad with UV coordinates in range `-1..=2`, which is outside of texture UV range `0..=1`.
fn quad_1_2() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[-1., 2.], [2., 2.], [2., -1.], [-1., -1.]],
    );
    mesh.set_indices(Some(Indices::U16(vec![0, 1, 2, 2, 3, 0])));
    mesh
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: ResMut<AssetServer>,
) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 2.,
                min_height: 1.,
            },
            far: 1000.,
            near: -1000.,
            ..default()
        },
        ..default()
    });

    // Texture from ambientCG.com, licensed under the Creative Commons CC0 1.0 Universal License.
    // https://ambientCG.com/a/Facade018A

    // By default Bevy loads images to textures with sampler settings that clamp the image to the edges.
    let image = asset_server.load("textures/facade018a.png");

    // Here we override the sampler settings to repeat the image instead.
    let image_repeat = asset_server.load_with_settings(
        // We are using another file name, because Bevy ignores different loader settings for the same file.
        // https://github.com/bevyengine/bevy/issues/11111
        "textures/facade018a_copy.png",
        |s: &mut ImageLoaderSettings| match &mut s.sampler {
            ImageSampler::Default => {
                s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    ..default()
                });
            }
            ImageSampler::Descriptor(sampler) => {
                sampler.address_mode_u = ImageAddressMode::Repeat;
                sampler.address_mode_v = ImageAddressMode::Repeat;
            }
        },
    );

    let mesh = meshes.add(quad_1_2());

    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh.clone().into(),
        material: materials.add(ColorMaterial {
            texture: Some(image),
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(-0.95, -0.45, 0.))
            .with_scale(Vec3::new(0.9, 0.9, 0.9)),
        ..default()
    });

    commands.spawn(MaterialMesh2dBundle {
        mesh: mesh.into(),
        material: materials.add(ColorMaterial {
            texture: Some(image_repeat),
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(0.05, -0.45, 0.))
            .with_scale(Vec3::new(0.9, 0.9, 0.9)),
        ..default()
    });
}
