//! Shows how to render to a double buffered texture that can then be used on the same render layer.

use bevy::prelude::*;
use bevy::render::camera::{CameraPlugin, RenderTarget};

use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

#[derive(Component, Default)]
pub struct CaptureCamera;

// Marks the first pass cube (rendered to a texture.)
#[derive(Component)]
struct FirstPassCube;

// Marks the main pass cube, to which the texture is applied.
#[derive(Component)]
struct MainPassCube;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 }) // Use 4x MSAA
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(CameraPlugin::default())
        .add_startup_system(setup)
        .add_system(cube_rotator_system)
        .add_system(rotator_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //let capture = FrameCapture::new_gpu_double_buffer(
    //    512,
    //    512,
    //    true,
    //    TextureFormat::Rgba8UnormSrgb,
    //    &mut images,
    //);

    //--------------------------------
    let width = 512;
    let height = 512;
    let size = Extent3d {
        width,
        height,
        ..Default::default()
    };

    // This is the texture that will be rendered to.
    let mut image1 = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..Default::default()
    };
    image1.resize(size);

    let gpu_image1 = images.add(image1);

    let mut image2 = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..Default::default()
    };
    image2.resize(size);

    let gpu_image2 = images.add(image2);
    //----------------------------

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 0.25 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // The cube that will be rendered to the texture.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.25, 0.0)),
            ..default()
        })
        .insert(FirstPassCube);

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    let cube_size = 0.25;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

    // This material has the texture that has been rendered.
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(gpu_image2.clone()),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });
    // Main pass cube, with material containing the rendered first pass texture.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.5, 0.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
                ..default()
            },
            ..default()
        })
        .insert(MainPassCube);

    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        })
        .with_children(|parent| {
            let render_target = RenderTarget::BufferedImage(gpu_image1.clone(), gpu_image2.clone());
            parent.spawn_bundle(Camera3dBundle {
                camera: Camera {
                    target: render_target,
                    ..default()
                },
                ..default()
            });
        });
}

/// Rotates the inner cube (first pass)
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<FirstPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.5 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_z(1.3 * time.delta_seconds());
    }
}

/// Rotates the outer cube (main pass)
fn cube_rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<MainPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.0 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_y(0.7 * time.delta_seconds());
    }
}
