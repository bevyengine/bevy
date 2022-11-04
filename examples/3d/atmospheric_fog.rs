//! This example showcases atmospheric fog

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(toggle_system)
        .add_startup_system(setup_camera_fog)
        .add_startup_system(setup_terrain_scene)
        .add_startup_system(setup_instructions)
        .run();
}

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-1.0, 0.1, 1.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        },
        FogSettings {
            color: Color::rgba(0.1, 0.2, 0.4, 1.0),
            directional_light_color: Color::rgba(1.0, 0.95, 0.75, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                15.0,
                Color::rgb(0.35, 0.5, 0.66),
                Color::rgb(0.8, 0.844, 1.0),
            ),
        },
    ));
}

fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Sun
    const HALF_SIZE: f32 = 1.5;
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -HALF_SIZE,
                far: HALF_SIZE,
                ..default()
            },
            color: Color::rgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
        ..default()
    });

    // Terrain
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/terrain/Mountains.gltf#Scene0"),
        ..default()
    });

    // Sky
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::default())),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("000000").unwrap(),
            emissive: Color::hex("888888").unwrap(),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            cull_mode: None,
            ..default()
        }),
        transform: Transform::from_scale(Vec3::splat(20.0)),
        ..default()
    });
}

fn setup_instructions(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((TextBundle::from_section(
        "Press Spacebar to Toggle Atmospheric Fog.\nPress S to Toggle Directional Light Fog Influence.",
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            font_size: 12.0,
            color: Color::WHITE,
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
    }),));
}

fn toggle_system(keycode: Res<Input<KeyCode>>, mut fog: Query<&mut FogSettings>) {
    let mut fog_settings = fog.single_mut();

    if keycode.just_pressed(KeyCode::Space) {
        let a = fog_settings.color.a();
        fog_settings.color.set_a(1.0 - a);
    }

    if keycode.just_pressed(KeyCode::S) {
        let a = fog_settings.directional_light_color.a();
        fog_settings.directional_light_color.set_a(0.5 - a);
    }
}
