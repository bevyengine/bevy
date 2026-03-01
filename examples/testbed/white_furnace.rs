//! This example demonstrates a white furnace test for Physically Based Rendering (PBR) materials.
//! A white furnace test uses a pure white environment map to verify that materials correctly
//! conserve energy and appear white when viewed under uniform white lighting.

use bevy::{
    asset::RenderAssetUsages,
    camera::{Hdr, ScalingMode},
    core_pipeline::{tonemapping::Tonemapping, Skybox},
    image::Image,
    light::GeneratedEnvironmentMapLight,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
    },
};

#[derive(Resource)]
struct LightConfigs {
    solid_color: EnvironmentMapLight,
    generated: GeneratedEnvironmentMapLight,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, switch_light_mode)
        .run();
}

/// Creates a pure white cubemap
fn create_white_cubemap(size: u32) -> Image {
    // f16 bytes for 1.0 (white): [0, 60] in little-endian
    const WHITE_F16: [u8; 2] = [0, 60];
    const WHITE_PIXEL: [u8; 8] = [
        WHITE_F16[0],
        WHITE_F16[1], // R
        WHITE_F16[0],
        WHITE_F16[1], // G
        WHITE_F16[0],
        WHITE_F16[1], // B
        WHITE_F16[0],
        WHITE_F16[1], // A
    ];

    let pixel_data: Vec<u8> = (0..6 * size * size).flat_map(|_| WHITE_PIXEL).collect();

    Image {
        texture_view_descriptor: Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        }),
        ..Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            pixel_data,
            TextureFormat::Rgba16Float,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let sphere_mesh = meshes.add(Sphere::new(0.45));
    // add entities to the world
    for y in -2..=2 {
        for x in -5..=5 {
            let x01 = (x + 5) as f32 / 10.0;
            let y01 = (y + 2) as f32 / 4.0;
            // sphere
            commands.spawn((
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: LinearRgba::WHITE.into(),
                    // vary key PBR parameters on a grid of spheres to show the effect
                    metallic: y01,
                    perceptual_roughness: x01,
                    ..default()
                })),
                Transform::from_xyz(x as f32, y as f32 + 0.5, 0.0),
            ));
        }
    }
    // unlit sphere
    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: LinearRgba::WHITE.into(),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(-5.0, -2.5, 0.0),
    ));

    // labels
    commands.spawn((
        Text::new("Perceptual Roughness"),
        TextFont {
            font_size: FontSize::Px(30.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(20),
            left: px(100),
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Metallic"),
        TextFont {
            font_size: FontSize::Px(30.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(130),
            right: Val::ZERO,
            ..default()
        },
        UiTransform {
            rotation: Rot2::degrees(90.),
            ..default()
        },
    ));

    // Create a pure white cubemap
    let white_cubemap = create_white_cubemap(256);
    let white_cubemap_handle = images.add(white_cubemap);

    let mut solid_color_light = EnvironmentMapLight::solid_color(&mut images, Color::WHITE);
    solid_color_light.intensity = 500.0;

    let generated_light = GeneratedEnvironmentMapLight {
        environment_map: white_cubemap_handle.clone(),
        intensity: 500.0,
        ..default()
    };

    commands.insert_resource(LightConfigs {
        solid_color: solid_color_light.clone(),
        generated: generated_light.clone(),
    });

    // camera
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::None,
        Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::default(), Vec3::Y),
        Projection::from(OrthographicProjection {
            scale: 0.01,
            scaling_mode: ScalingMode::WindowSize,
            ..OrthographicProjection::default_3d()
        }),
        Skybox {
            image: white_cubemap_handle.clone(),
            // middle gray
            brightness: 500.0,
            ..default()
        },
        solid_color_light,
    ));

    // usage instructions
    commands.spawn((
        Text::new(
            "Press '1' for solid color light\n\
             Press '2' for generated environment map light",
        ),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn switch_light_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    light_configs: Res<LightConfigs>,
    mut commands: Commands,
    camera_query: Query<Entity, With<Camera3d>>,
) {
    for camera in &camera_query {
        let mut entity = commands.entity(camera);
        if keyboard.just_pressed(KeyCode::Digit1) {
            entity
                .remove::<(EnvironmentMapLight, GeneratedEnvironmentMapLight)>()
                .insert(light_configs.solid_color.clone());
        } else if keyboard.just_pressed(KeyCode::Digit2) {
            entity
                .remove::<(EnvironmentMapLight, GeneratedEnvironmentMapLight)>()
                .insert(light_configs.generated.clone());
        }
    }
}
