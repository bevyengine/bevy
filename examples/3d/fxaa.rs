//! This examples compares MSAA (Multi-Sample Anti-Aliasing) and FXAA (Fast Approximate Anti-Aliasing).

use std::f32::consts::PI;

use bevy::{
    core_pipeline::fxaa::{Fxaa, Quality},
    prelude::*,
    render::{
        render_resource::{Extent3d, SamplerDescriptor, TextureDimension, TextureFormat},
        texture::ImageSampler,
    },
};

fn main() {
    App::new()
        // Disable MSAA be default
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(toggle_fxaa)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    println!("Toggle with:");
    println!("1 - NO AA");
    println!("2 - MSAA 4");
    println!("3 - FXAA (default)");

    println!("Threshold:");
    println!("7 - LOW");
    println!("8 - MEDIUM");
    println!("9 - HIGH (default)");
    println!("0 - ULTRA");

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // cubes
    for i in 0..5 {
        commands.spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.25 })),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(images.add(uv_debug_texture())),
                ..default()
            }),
            transform: Transform::from_xyz(i as f32 * 0.25 - 1.0, 0.125, -i as f32 * 0.5),
            ..default()
        });
    }

    // Flight Helmet
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
        ..default()
    });

    // light
    const HALF_SIZE: f32 = 2.0;
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            PI * -0.15,
            PI * -0.15,
        )),
        ..default()
    });

    // camera
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: false, // Works with and without hdr
                ..default()
            },
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        })
        .insert(Fxaa::default());
}

fn toggle_fxaa(keys: Res<Input<KeyCode>>, mut query: Query<&mut Fxaa>, mut msaa: ResMut<Msaa>) {
    let set_no_aa = keys.just_pressed(KeyCode::Key1);
    let set_msaa = keys.just_pressed(KeyCode::Key2);
    let set_fxaa = keys.just_pressed(KeyCode::Key3);
    let fxaa_low = keys.just_pressed(KeyCode::Key7);
    let fxaa_med = keys.just_pressed(KeyCode::Key8);
    let fxaa_high = keys.just_pressed(KeyCode::Key9);
    let fxaa_ultra = keys.just_pressed(KeyCode::Key0);
    let set_fxaa = set_fxaa | fxaa_low | fxaa_med | fxaa_high | fxaa_ultra;
    for mut fxaa in &mut query {
        if set_msaa {
            fxaa.enabled = false;
            msaa.samples = 4;
            info!("MSAA 4x");
        }
        if set_no_aa {
            fxaa.enabled = false;
            msaa.samples = 1;
            info!("NO AA");
        }
        if set_no_aa | set_fxaa {
            msaa.samples = 1;
        }
        if fxaa_low {
            fxaa.edge_threshold = Quality::Low;
            fxaa.edge_threshold_min = Quality::Low;
        } else if fxaa_med {
            fxaa.edge_threshold = Quality::Medium;
            fxaa.edge_threshold_min = Quality::Medium;
        } else if fxaa_high {
            fxaa.edge_threshold = Quality::High;
            fxaa.edge_threshold_min = Quality::High;
        } else if fxaa_ultra {
            fxaa.edge_threshold = Quality::Ultra;
            fxaa.edge_threshold_min = Quality::Ultra;
        }
        if set_fxaa {
            fxaa.enabled = true;
            msaa.samples = 1;
            info!("FXAA {}", fxaa.edge_threshold.get_str());
        }
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    let mut img = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    img.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor::default());
    img
}
