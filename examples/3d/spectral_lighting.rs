//! Showcases support for spectral color (monochromatic) lights in a 3D scene.
//!
//! ## Controls
//!
//! | Key Binding        | Action                                               |
//! |:-------------------|:-----------------------------------------------------|
//! | Left/Right Arrows  | Change wavelength                                    |
//! | Up/Down Arrows     | Change monochromaticity                              |
//! | Space              | Toggle monochromaticity                              |

use bevy::{
    color::palettes::css::{ANTIQUE_WHITE, WHITE},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .insert_resource(AmbientLight {
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
struct Polychromatic;

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
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    // red sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(1.0)),
        material: materials.add(Color::srgb(1.0, 0.0, 0.0)),
        transform: Transform::from_xyz(-2.0, 1.0, 0.0),
        ..default()
    });
    // green sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(1.0)),
        material: materials.add(Color::srgb(0.0, 1.0, 0.0)),
        transform: Transform::from_xyz(0.0, 1.0, 0.0),
        ..default()
    });
    // blue sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(1.0)),
        material: materials.add(Color::srgb(0.0, 0.0, 1.0)),
        transform: Transform::from_xyz(2.0, 1.0, 0.0),
        ..default()
    });
    // rainbow cubes
    for j in 0..=5 {
        for i in 0..=17 {
            // cube
            commands.spawn(PbrBundle {
                mesh: meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
                material: materials.add(StandardMaterial {
                    base_color: if j == 5 {
                        Color::hsva(0.0, 0.0, i as f32 / 17.0, 1.0)
                    } else {
                        Color::hsva(i as f32 * 15.0, j as f32 / 4.0, 1.0, 1.0)
                    },
                    perceptual_roughness: 1.0,
                    ..default()
                }),
                transform: Transform::from_xyz(-4.0 + i as f32 * 0.5, 2.5 + j as f32 * 0.5, 0.0),
                ..default()
            });
        }
    }
    // monochromatic light
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                color: SpectralColor::INFRARED.into(),
                shadows_enabled: true,
                #[cfg(feature = "spectral_lighting")]
                monochromaticity: 1.0,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        },
        Monochromatic,
    ));
    // polychromatic light
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                intensity: 1000000.0,
                ..default()
            },
            transform: Transform::from_xyz(-5.0, 2.5, 0.0),
            ..default()
        },
        Polychromatic,
    ));
    // camera
    commands.spawn(Camera3dBundle {
        camera: Camera { ..default() },
        transform: Transform::from_xyz(-3.5, 7.0, 12.0)
            .looking_at(Vec3::new(0.0, 2.5, 0.), Vec3::Y),
        ..default()
    });
    // UI
    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
    commands
        .spawn(ImageBundle {
            style: Style {
                width: Val::Px(360.),
                height: Val::Px(68.5),
                position_type: PositionType::Absolute,
                top: Val::Px(70.),
                left: Val::Px(12.),
                ..default()
            },
            image: UiImage::new(asset_server.load("textures/Linear_visible_spectrum.png")),
            background_color: BackgroundColor(ANTIQUE_WHITE.into()),
            ..default()
        })
        .with_children(|children| {
            children.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(4.),
                        height: Val::Px(40.),
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(1.),
                        left: Val::Px(0.),
                        ..default()
                    },
                    border_radius: BorderRadius::all(Val::Px(2.)),
                    background_color: BackgroundColor(WHITE.into()),
                    ..default()
                },
                Indicator,
            ));
        });
}

fn update(
    mut text_query: Query<&mut Text>,
    mut polychromatic_light_query: Query<&mut Transform, With<Polychromatic>>,
    mut indicator_query: Query<&mut Style, With<Indicator>>,
    mut monochromatic_light_query: Query<&mut PointLight, With<Monochromatic>>,
    mut config: ResMut<Config>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let mut monochromatic_light = monochromatic_light_query.single_mut();
    let mut polychromatic_light = polychromatic_light_query.single_mut();
    let mut text = text_query.single_mut();
    let mut indicator = indicator_query.single_mut();

    text.sections[0].value = format!(
        "Light Wavelength: {:.0} nm (Left/Right Arrows to Adjust)\nLight Monochromaticity: {:.2} (Up/Down Arrows to Adjust, Space to Toggle)",
        config.spectral_color.wavelength, monochromatic_light.monochromaticity
    );

    indicator.left = Val::Px(356. * (config.spectral_color.wavelength - 380.) / (750. - 380.));

    polychromatic_light.translation = Vec3::new(
        -5.0 * time.elapsed_seconds().cos(),
        2.5,
        5.0 * time.elapsed_seconds().sin(),
    );

    if keyboard.pressed(KeyCode::ArrowRight) {
        config.spectral_color.wavelength += time.delta_seconds() * 100.0;
        config.spectral_color.wavelength = config.spectral_color.wavelength.min(750.0);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.spectral_color.wavelength -= time.delta_seconds() * 100.0;
        config.spectral_color.wavelength = config.spectral_color.wavelength.max(380.0);
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        monochromatic_light.monochromaticity += time.delta_seconds();
        monochromatic_light.monochromaticity = monochromatic_light.monochromaticity.min(1.0);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        monochromatic_light.monochromaticity -= time.delta_seconds();
        monochromatic_light.monochromaticity = monochromatic_light.monochromaticity.max(0.0);
    }
    if keyboard.just_pressed(KeyCode::Space) {
        monochromatic_light.monochromaticity = (1.0 - monochromatic_light.monochromaticity).round();
    }

    monochromatic_light.color = config.spectral_color.into();
}
