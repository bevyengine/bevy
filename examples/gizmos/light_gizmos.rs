//! This example demonstrates how to visualize lights properties through the gizmo API.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    color::palettes::css::{DARK_CYAN, GOLD, GRAY, PURPLE},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .add_systems(Update, update_config)
        .run();
}

#[derive(Component)]
struct GizmoColorText;

fn gizmo_color_text(config: &LightGizmoConfigGroup) -> String {
    match config.color {
        LightGizmoColor::Manual(color) => format!("Manual {}", Srgba::from(color).to_hex()),
        LightGizmoColor::Varied => "Random from entity".to_owned(),
        LightGizmoColor::MatchLightColor => "Match light color".to_owned(),
        LightGizmoColor::ByLightType => {
            format!(
                "Point {}, Spot {}, Directional {}",
                Srgba::from(config.point_light_color).to_hex(),
                Srgba::from(config.spot_light_color).to_hex(),
                Srgba::from(config.directional_light_color).to_hex()
            )
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    // Circular base.
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ..default()
    });

    // Cubes.
    {
        let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let material = materials.add(Color::srgb_u8(124, 144, 255));
        for x in [-2.0, 0.0, 2.0] {
            commands.spawn(PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_xyz(x, 0.5, 0.0),
                ..default()
            });
        }
    }

    // Lights.
    {
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                range: 2.0,
                color: DARK_CYAN.into(),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 1.5, 0.0),
            ..default()
        });
        commands.spawn(SpotLightBundle {
            spot_light: SpotLight {
                shadows_enabled: true,
                range: 3.5,
                color: PURPLE.into(),
                outer_angle: PI / 4.0,
                inner_angle: PI / 4.0 * 0.8,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 2.0, 0.0).looking_at(Vec3::X * 1.5, Vec3::Y),
            ..default()
        });
        commands.spawn(DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: GOLD.into(),
                illuminance: DirectionalLight::default().illuminance * 0.05,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(-4.0, 2.0, 0.0).looking_at(Vec3::NEG_X * 1.5, Vec3::Y),
            ..default()
        });
    }

    // Camera.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Example instructions and gizmo config.
    {
        let text_style = TextStyle::default();

        commands.spawn(
            TextBundle::from_section(
                "Press 'D' to toggle drawing gizmos on top of everything else in the scene\n\
            Hold 'Left' or 'Right' to change the line width of the gizmos\n\
            Press 'A' to toggle drawing of the light gizmos\n\
            Press 'C' to cycle between the light gizmos coloring modes",
                text_style.clone(),
            )
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            }),
        );

        let (_, light_config) = config_store.config_mut::<LightGizmoConfigGroup>();
        light_config.draw_all = true;
        light_config.color = LightGizmoColor::MatchLightColor;

        commands.spawn((
            TextBundle::from_sections([
                TextSection::new("Gizmo color mode: ", text_style.clone()),
                TextSection::new(gizmo_color_text(light_config), text_style),
            ])
            .with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            }),
            GizmoColorText,
        ));
    }
}

fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let mut transform = query.single_mut();

    transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds() / 2.));
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut color_text_query: Query<&mut Text, With<GizmoColorText>>,
) {
    if keyboard.just_pressed(KeyCode::KeyD) {
        for (_, config, _) in config_store.iter_mut() {
            config.depth_bias = if config.depth_bias == 0. { -1. } else { 0. };
        }
    }

    let (config, light_config) = config_store.config_mut::<LightGizmoConfigGroup>();
    if keyboard.pressed(KeyCode::ArrowRight) {
        config.line_width += 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.line_width -= 5. * time.delta_seconds();
        config.line_width = config.line_width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::KeyA) {
        config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyC) {
        light_config.color = match light_config.color {
            LightGizmoColor::Manual(_) => LightGizmoColor::Varied,
            LightGizmoColor::Varied => LightGizmoColor::MatchLightColor,
            LightGizmoColor::MatchLightColor => LightGizmoColor::ByLightType,
            LightGizmoColor::ByLightType => LightGizmoColor::Manual(GRAY.into()),
        };
        color_text_query.single_mut().sections[1].value = gizmo_color_text(light_config);
    }
}
