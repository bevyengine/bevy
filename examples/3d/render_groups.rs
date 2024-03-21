//! Load a scene from a glTF file and render it with different render groups.

use bevy::{
    color::palettes,
    pbr::DirectionalLightShadowMap,
    prelude::*,
    render::view::{CameraView, PropagateRenderGroups, RenderGroups},
};

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_layers)
        .run();
}

#[derive(Component)]
struct MovedScene;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        DirectionalLightBundle {
            transform: Transform::from_xyz(4.0, 25.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            directional_light: DirectionalLight {
                shadows_enabled: true,
                illuminance: 100000.0,
                color: palettes::basic::RED.into(),
                ..default()
            },
            ..default()
        },
        RenderGroups::from_layers(&[1, 2, 3]),
    ));

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0., 1.4, 2.0)
                .looking_at(Vec3::new(0., 0.3, 0.0), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1500.0,
        },
        CameraView::from_layers(&[0, 1, 2, 3]),
    ));

    commands.spawn((
        TextBundle::from_section(
            "Press '1..3' to toggle camera render layers\n\
            Press '4..6' to toggle directional light render layers",
            TextStyle {
                font_size: 20.,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
        RenderGroups::from_layer(0),
    ));

    // Spawn three copies of the scene, each with a different render group.
    for i in 0..3 {
        commands.spawn((
            SceneBundle {
                transform: Transform::from_xyz(i as f32 - 1.0, 0.0, 0.0),
                scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
                ..default()
            },
            RenderGroups::from_layer(i + 1),
            PropagateRenderGroups::Auto,
        ));
    }
}

fn toggle_layers(
    mut query_camera: Query<&mut CameraView>,
    mut query_light: Query<&mut RenderGroups, With<DirectionalLight>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut camera_view) = query_camera.get_single_mut() else {
        return;
    };

    let Ok(mut light_groups) = query_light.get_single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Digit1) {
        toggle_camera_layer(&mut camera_view, 1);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        toggle_camera_layer(&mut camera_view, 2);
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        toggle_camera_layer(&mut camera_view, 3);
    }

    if keyboard.just_pressed(KeyCode::Digit4) {
        toggle_render_layer(&mut light_groups, 1);
    }
    if keyboard.just_pressed(KeyCode::Digit5) {
        toggle_render_layer(&mut light_groups, 2);
    }
    if keyboard.just_pressed(KeyCode::Digit6) {
        toggle_render_layer(&mut light_groups, 3);
    }
}

fn toggle_camera_layer(camera_view: &mut CameraView, layer: usize) {
    if camera_view.contains_layer(layer) {
        camera_view.remove(layer);
    } else {
        camera_view.add(layer);
    }
}

fn toggle_render_layer(groups: &mut RenderGroups, layer: usize) {
    if groups.contains_layer(layer) {
        groups.remove(layer);
    } else {
        groups.add(layer);
    }
}
