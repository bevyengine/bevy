//! A simple 3D scene with light shining over a cube sitting on a plane.

use bevy::prelude::*;
use bevy_ecs::{template::system_value};

fn main() {
    fn run_callbacks(mut commands: Commands, callbacks: Query<(Entity,&Callback)>) {
        callbacks.iter().for_each(|(entity, callback)| {
            commands.run_system(callback.0);

            commands.entity(entity).despawn();
        });
    }
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .add_systems(Startup, (callback_scene.spawn(), run_callbacks, callback_scene.spawn(), run_callbacks).chain())
        .run();
}

/// set up a simple 3D scene
fn scene() -> impl SceneList {
    bsn_list! [
        (
            #CircularBase
            Mesh3d(asset_value(Circle::new(4.0)))
            MeshMaterial3d::<StandardMaterial>(asset_value(Color::WHITE))
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
        ),
        (
            #Cube
            Mesh3d(asset_value(Cuboid::new(1.0, 1.0, 1.0)))
            MeshMaterial3d::<StandardMaterial>(asset_value(Color::srgb_u8(124, 144, 255)))
            Transform::from_xyz(0.0, 0.5, 0.0)
        ),
        (
            PointLight {
                shadow_maps_enabled: true,
            }
            Transform::from_xyz(4.0, 8.0, 4.0)
        ),
        (
            Camera3d
            template_value(Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y))
        ),
    ]
}

#[derive(Component, FromTemplate)]
struct Callback(bevy_ecs::system::SystemId);
fn callback_scene() -> impl SceneList {
    bsn_list! {
        Name("template1") Callback(system_value(callback_system)),
        Name("template2") Callback(system_value(callback_system)),
    }
}
fn callback_system(mut call_counter:Local<u32>){
    *call_counter += 1;
    println!("Hello from the system! Called: {}", *call_counter);
}
