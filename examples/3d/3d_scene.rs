//! A simple 3D scene with light shining over a cube sitting on a plane.

use bevy::{
    camera::visibility::{NoCpuCulling, RenderLayers},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, (change_layers, change_light_layers))
        .add_observer(add_ncl)
        .run();
}

/// set up a simple 3D scene
fn scene() -> impl SceneList {
    bsn! {
        #CircularBase
        Mesh3d(asset_value(Circle::new(4.0)))
        MeshMaterial3d::<StandardMaterial>(asset_value(Color::WHITE))
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
        --
        #Cube
        Mesh3d(asset_value(Cuboid::new(1.0, 1.0, 1.0)))
        MeshMaterial3d::<StandardMaterial>(asset_value(Color::srgb_u8(124, 144, 255)))
        Transform::from_xyz(0.0, 0.5, 0.0)
        --
        PointLight {
            shadow_maps_enabled: true,
        }
        Transform::from_xyz(4.0, 8.0, 4.0)
        --
        Camera3d
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y)
    }
}

fn add_ncl(add: On<Add<Mesh3d>>, mut commands: Commands) {
    commands.entity(add.entity).insert(NoCpuCulling);
}

fn change_layers(
    keyboard: Res<ButtonInput<KeyCode>>,
    query: Query<Entity, With<Camera3d>>,
    mut commands: Commands,
    mut state: Local<bool>,
) {
    if keyboard.just_pressed(KeyCode::KeyP) {
        let layers = if *state {
            RenderLayers::layer(0)
        } else {
            RenderLayers::layer(1)
        };
        *state = !*state;
        for entity in query {
            commands.entity(entity).insert(layers.clone());
        }
    }
}

fn change_light_layers(
    keyboard: Res<ButtonInput<KeyCode>>,
    query: Query<Entity, Or<(With<PointLight>, With<SpotLight>, With<DirectionalLight>)>>,
    mut commands: Commands,
    mut state: Local<bool>,
) {
    if keyboard.just_pressed(KeyCode::KeyL) {
        let layers = if *state {
            RenderLayers::layer(0)
        } else {
            RenderLayers::layer(1)
        };
        *state = !*state;
        for entity in query {
            commands.entity(entity).insert(layers.clone());
        }
    }
}
