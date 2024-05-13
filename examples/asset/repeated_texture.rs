//! By default Bevy loads images to textures that clamps the image to the edges
//! This example shows how to configure it to repeat the image instead.

use bevy::{
    math::Affine2,
    prelude::*,
    render::texture::{
        ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let image_with_default_sampler =
        asset_server.load("textures/fantasy_ui_borders/panel-border-010.png");

    // central cube with not repeated texture
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(image_with_default_sampler.clone()),
            ..default()
        }),
        transform: Transform::from_translation(Vec3::ZERO),
        ..default()
    });

    // left cube with repeated texture
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load_with_settings(
                "textures/fantasy_ui_borders/panel-border-010-repeated.png",
                |s: &mut _| {
                    *s = ImageLoaderSettings {
                        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                            // rewriting mode to repeat image,
                            address_mode_u: ImageAddressMode::Repeat,
                            address_mode_v: ImageAddressMode::Repeat,
                            ..default()
                        }),
                        ..default()
                    }
                },
            )),

            // uv_transform used here for proportions only, but it is full Affine2
            // that's why you can use rotation and shift also
            uv_transform: Affine2::from_scale(Vec2::new(2., 3.)),
            ..default()
        }),
        transform: Transform::from_xyz(-1.5, 0.0, 0.0),
        ..default()
    });

    // right cube with scaled texture, because with default sampler
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(StandardMaterial {
            // there is no sampler set, that's why
            // by default you see only one small image in a row/column
            // and other space is filled by image edge
            base_color_texture: Some(image_with_default_sampler),

            // uv_transform used here for proportions only, but it is full Affine2
            // that's why you can use rotation and shift also
            uv_transform: Affine2::from_scale(Vec2::new(2., 3.)),
            ..default()
        }),
        transform: Transform::from_xyz(1.5, 0.0, 0.0),
        ..default()
    });

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.5, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
