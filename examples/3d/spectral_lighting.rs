//! Showcases support for spectral color (monochromatic) lights in a 3D scene.
//!
//! ## Controls
//!
//! | Key Binding        | Action                                               |
//! |:-------------------|:-----------------------------------------------------|
//! | Left/Right Arrows  | Change wavelength                                    |
//! | Space              | Toggle monochromatic                                 |

use bevy::{color::palettes::css::WHITE, color::SpectralColor, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .insert_resource(GlobalAmbientLight {
            brightness: 0.0,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .insert_resource(Config {
            spectral_color: SpectralColor::SODIUM_VAPOR,
        })
        .run();
}

#[derive(Resource)]
struct Config {
    spectral_color: SpectralColor,
}

#[derive(Component)]
struct Monochromatic;

#[derive(Component)]
struct Indicator;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // red sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
        Transform::from_xyz(-2.0, 1.0, 0.0),
    ));
    // green sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
    // blue sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 0.0, 1.0))),
        Transform::from_xyz(2.0, 1.0, 0.0),
    ));
    // HSV cubes
    for j in 0..=5 {
        for i in 0..=17 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: if j == 5 {
                        // Grayscale band at the top
                        Color::hsv(0.0, 0.0, i as f32 / 17.0)
                    } else {
                        Color::hsv(255.0 - i as f32 * 15.0, j as f32 / 4.0, 1.0)
                    },
                    perceptual_roughness: 1.0,
                    reflectance: 0.0,
                    ..default()
                })),
                Transform::from_xyz(-4.0 + i as f32 * 0.5, 2.5 + j as f32 * 0.5, 0.0),
            ));
        }
    }
    // monochromatic light
    commands.spawn((
        PointLight {
            color: SpectralColor::SODIUM_VAPOR.into(),
            shadow_maps_enabled: true,
            #[cfg(feature = "spectral_lighting")]
            monochromatic: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        Monochromatic,
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-3.5, 7.0, 12.0).looking_at(Vec3::new(0.0, 2.5, 0.), Vec3::Y),
    ));
    // UI
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::all(px(5)),
            ..default()
        },
        children![(Text::default(), children![TextSpan::new("")])],
    ));
    commands.spawn((
        Node {
            width: px(360.),
            height: px(68.5),
            position_type: PositionType::Absolute,
            top: px(70.),
            left: px(12.),
            ..default()
        },
        ImageNode::new(asset_server.load("textures/Linear_visible_spectrum.png")),
        BackgroundColor(WHITE.into()),
        children![(
            Node {
                width: px(4.),
                height: px(40.),
                position_type: PositionType::Absolute,
                bottom: px(1.),
                left: px(0.),
                border_radius: BorderRadius::all(px(2.)),
                ..default()
            },
            BackgroundColor(WHITE.into()),
            Indicator,
        )],
    ));
}

fn update(
    mut text_query: Query<&mut TextSpan>,
    mut indicator_query: Query<&mut Node, With<Indicator>>,
    mut monochromatic_light_query: Query<&mut PointLight, With<Monochromatic>>,
    mut config: ResMut<Config>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
) -> Result {
    let mut monochromatic_light = monochromatic_light_query.single_mut()?;
    let mut text = text_query.single_mut()?;
    let mut indicator = indicator_query.single_mut()?;

    text.0 = format!(
        "Light Wavelength: {:.0} nm (Left/Right Arrows to Adjust)\nLight Monochromatic: {:?} (Space to Toggle)",
        config.spectral_color.wavelength, monochromatic_light.monochromatic
    );

    indicator.left = px(356. * (config.spectral_color.wavelength - 380.) / (750. - 380.));

    if keyboard.pressed(KeyCode::ArrowRight) {
        config.spectral_color.wavelength += time.delta_secs() * 100.0;
        config.spectral_color.wavelength = config.spectral_color.wavelength.min(750.0);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.spectral_color.wavelength -= time.delta_secs() * 100.0;
        config.spectral_color.wavelength = config.spectral_color.wavelength.max(380.0);
    }
    if keyboard.just_pressed(KeyCode::Space) {
        monochromatic_light.monochromatic = !monochromatic_light.monochromatic;
    }

    monochromatic_light.color = config.spectral_color.into();

    Ok(())
}
