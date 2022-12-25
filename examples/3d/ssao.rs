//! A scene showcasing screen space ambient occlusion.

use bevy::{
    pbr::{
        ScreenSpaceAmbientOcclusionBundle, ScreenSpaceAmbientOcclusionSettings,
        TemporalAntialiasBundle, TemporalAntialiasPlugin,
    },
    prelude::*,
    render::camera::TemporalJitter,
};

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            brightness: 5.0,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(TemporalAntialiasPlugin)
        .add_startup_system(setup)
        .add_system(update)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.0, -2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        ScreenSpaceAmbientOcclusionBundle::default(),
        // TemporalAntialiasBundle::default(),
    ));

    let material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.5, 0.5, 0.5),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: material.clone(),
        transform: Transform::from_xyz(0.0, 0.0, 1.0),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: material.clone(),
        transform: Transform::from_xyz(0.0, -1.0, 0.0),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material,
        transform: Transform::from_xyz(1.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.4,
                ..default()
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.4, 0.4, 0.4),
                perceptual_roughness: 1.0,
                reflectance: 0.0,
                ..default()
            }),
            ..default()
        },
        SphereMarker,
    ));

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(3.0, 2.0, 0.5),
        ..default()
    });

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
    camera: Query<
        (
            Entity,
            Option<&ScreenSpaceAmbientOcclusionSettings>,
            Option<&TemporalJitter>,
        ),
        With<Camera>,
    >,
    mut text: Query<&mut Text>,
    mut sphere: Query<&mut Transform, With<SphereMarker>>,
    mut commands: Commands,
    keycode: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut sphere = sphere.single_mut();
    sphere.translation.y = (time.elapsed_seconds() / 1.7).sin() * 0.7;

    let (camera_entity, ssao_settings, temporal_jitter) = camera.single();

    let mut commands = commands.entity(camera_entity);
    if keycode.just_pressed(KeyCode::Key1) {
        commands.remove::<ScreenSpaceAmbientOcclusionSettings>();
    }
    if keycode.just_pressed(KeyCode::Key2) {
        commands.insert(ScreenSpaceAmbientOcclusionSettings::Low);
    }
    if keycode.just_pressed(KeyCode::Key3) {
        commands.insert(ScreenSpaceAmbientOcclusionSettings::Medium);
    }
    if keycode.just_pressed(KeyCode::Key4) {
        commands.insert(ScreenSpaceAmbientOcclusionSettings::High);
    }
    if keycode.just_pressed(KeyCode::Key5) {
        commands.insert(ScreenSpaceAmbientOcclusionSettings::Ultra);
    }
    if keycode.just_pressed(KeyCode::Space) {
        if temporal_jitter.is_some() {
            commands.remove::<TemporalAntialiasBundle>();
        } else {
            commands.insert(TemporalAntialiasBundle::default());
        }
    }

    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;
    text.clear();

    let (o, l, m, h, u) = match ssao_settings {
        None => ("*", "", "", "", ""),
        Some(ScreenSpaceAmbientOcclusionSettings::Low) => ("", "*", "", "", ""),
        Some(ScreenSpaceAmbientOcclusionSettings::Medium) => ("", "", "*", "", ""),
        Some(ScreenSpaceAmbientOcclusionSettings::High) => ("", "", "", "*", ""),
        Some(ScreenSpaceAmbientOcclusionSettings::Ultra) => ("", "", "", "", "*"),
        _ => unreachable!(),
    };

    text.push_str("SSAO Quality:\n");
    text.push_str(&format!("(1) {o}Off{o}\n"));
    text.push_str(&format!("(2) {l}Low{l}\n"));
    text.push_str(&format!("(3) {m}Medium{m}\n"));
    text.push_str(&format!("(4) {h}High{h}\n"));
    text.push_str(&format!("(5) {u}Ultra{u}\n\n"));

    text.push_str("Temporal Antialiasing:\n");
    text.push_str(match temporal_jitter {
        Some(_) => "(Space) Enabled",
        None => "(Space) Disabled",
    });
}

#[derive(Component)]
struct SphereMarker;
