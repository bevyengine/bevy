//! By default Bevy loads images to textures that clamps the image to the edges
//! This example shows how to configure it to repeat the image instead.

use bevy::{
    audio::AudioPlugin,
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    math::Affine2,
    prelude::*,
};

/// How much to move some rectangles away from the center
const RECTANGLE_OFFSET: f32 = 250.0;
/// Length of the sides of the rectangle
const RECTANGLE_SIDE: f32 = 200.;
/// How much to move the label away from the rectangle
const LABEL_OFFSET: f32 = (RECTANGLE_SIDE / 2.) + 25.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.build().disable::<AudioPlugin>())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // #11111: We use a duplicated image so that it can be load with and without
    // settings
    let image_with_default_sampler =
        asset_server.load("textures/fantasy_ui_borders/panel-border-010.png");
    let image_with_repeated_sampler = asset_server.load_with_settings(
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
    );

    // central rectangle with not repeated texture
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(RECTANGLE_SIDE, RECTANGLE_SIDE))),
        MeshMaterial2d(materials.add(ColorMaterial {
            texture: Some(image_with_default_sampler.clone()),
            ..default()
        })),
        Transform::from_translation(Vec3::ZERO),
        children![(
            Text2d::new("Control"),
            Transform::from_xyz(0., LABEL_OFFSET, 0.),
        )],
    ));

    // left rectangle with repeated texture
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(RECTANGLE_SIDE, RECTANGLE_SIDE))),
        MeshMaterial2d(materials.add(ColorMaterial {
            texture: Some(image_with_repeated_sampler),
            // uv_transform used here for proportions only, but it is full Affine2
            // that's why you can use rotation and shift also
            uv_transform: Affine2::from_scale(Vec2::new(2., 3.)),
            ..default()
        })),
        Transform::from_xyz(-RECTANGLE_OFFSET, 0.0, 0.0),
        children![(
            Text2d::new("Repeat On"),
            Transform::from_xyz(0., LABEL_OFFSET, 0.),
        )],
    ));

    // right rectangle with scaled texture, but with default sampler.
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(RECTANGLE_SIDE, RECTANGLE_SIDE))),
        MeshMaterial2d(materials.add(ColorMaterial {
            // there is no sampler set, that's why
            // by default you see only one small image in a row/column
            // and other space is filled by image edge
            texture: Some(image_with_default_sampler),

            // uv_transform used here for proportions only, but it is full Affine2
            // that's why you can use rotation and shift also
            uv_transform: Affine2::from_scale(Vec2::new(2., 3.)),
            ..default()
        })),
        Transform::from_xyz(RECTANGLE_OFFSET, 0.0, 0.0),
        children![(
            Text2d::new("Repeat Off"),
            Transform::from_xyz(0., LABEL_OFFSET, 0.),
        )],
    ));

    // camera
    commands.spawn((
        Camera2d,
        Transform::default().looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
