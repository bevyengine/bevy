//! This example showcases atmospheric fog
//!
//! ## Controls
//!
//! | Key Binding        | Action                                 |
//! |:-------------------|:---------------------------------------|
//! | `Spacebar`         | Toggle Atmospheric Fog                 |
//! | `S`                | Toggle Directional Light Fog Influence |

use bevy::{
    pbr::{CascadeShadowConfigBuilder, NotShadowCaster},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (setup_camera_fog, setup_terrain_scene, setup_instructions),
        )
        .add_systems(Update, toggle_system)
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
                15.0, // distance in world units up to which objects retain visibility (>= 5% contrast)
                Color::rgb(0.35, 0.5, 0.66), // atmospheric extinction color (after light is lost due to absorption by atmospheric particles)
                Color::rgb(0.8, 0.844, 1.0), // atmospheric inscattering color (light gained due to scattering from the sun)
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
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    // Sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
        cascade_shadow_config,
        ..default()
    });

    // Terrain
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/terrain/Mountains.gltf#Scene0"),
        ..default()
    });

    // Sky
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box::default())),
            material: materials.add(StandardMaterial {
                base_color: Color::hex("888888").unwrap(),
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(20.0)),
            ..default()
        },
        NotShadowCaster,
    ));
}

fn setup_instructions(mut commands: Commands) {
    commands.spawn((TextBundle::from_section(
        "Press Spacebar to Toggle Atmospheric Fog.\nPress S to Toggle Directional Light Fog Influence.",
        TextStyle {
            font_size: 20.0,
            color: Color::WHITE,
            ..default()
        },
    )
    .with_style(Style {
        position_type: PositionType::Absolute,
        bottom: Val::Px(12.0),
        left: Val::Px(12.0),
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
