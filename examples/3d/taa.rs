//! A scene showcasing temporal antialiasing.

use bevy::{
    core_pipeline::taa::{
        TemporalAntialiasBundle, TemporalAntialiasPlugin, TemporalAntialiasSettings,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TemporalAntialiasPlugin) // 1. Add the TAA plugin (this will disable MSAA)
        .add_startup_system(setup)
        .add_system(update)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        // 2. Add TemporalAntialiasBundle to the camera (must use a perspective projection)
        TemporalAntialiasBundle::default(),
    ));

    // TODO: Add moving object and camera

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // text
    commands.spawn(
        TextBundle::from_section(
            "",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 26.0,
                color: Color::BLACK,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        }),
    );
}

fn update(
    camera: Query<(Entity, Option<&TemporalAntialiasSettings>), With<Camera>>,
    mut text: Query<&mut Text>,
    mut commands: Commands,
    keycode: Res<Input<KeyCode>>,
) {
    let (camera_entity, taa_settings) = camera.single();

    if keycode.just_pressed(KeyCode::Space) {
        if taa_settings.is_some() {
            commands
                .entity(camera_entity)
                .remove::<TemporalAntialiasBundle>();
        } else {
            commands
                .entity(camera_entity)
                .insert(TemporalAntialiasBundle::default());
        }
    }

    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;
    text.clear();

    text.push_str("Temporal Antialiasing:\n");
    text.push_str(match taa_settings {
        Some(_) => "(Space) Enabled",
        None => "(Space) Disabled",
    });
}
