//! Controls morph targets in a loaded scene.
//!
//! Illustrates:
//!
//! - How to access and modify individual morph target weights.
//!   See the [`update_weights`] system for details.
//! - How to read morph target names in [`name_morphs`].
//! - How to play morph target animations in [`setup_animations`].
use std::f32::consts::PI;

use bevy::{gltf::Gltf, prelude::*};
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "morph targets".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(AmbientLight {
            brightness: 1.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (name_morphs, setup_animations))
        .run();
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/animated/MorphStressTest.gltf#Scene0"),
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::WHITE,
            illuminance: 19350.0,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_z(PI / 2.0)),
        ..default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3.0, 2.1, 10.2).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

/// You can get the target names in the `MorphTargetNames` component.
/// They are in the order of the weights.
fn name_morphs(mut has_printed: Local<bool>, meshes: Res<Assets<Mesh>>) {
    if *has_printed {
        return;
    }
    for (handle, mesh) in meshes.iter() {
        let Some(names) = mesh.morph_target_names() else { continue };
        println!("Morph Targets for {handle:?}:");
        for name in names {
            println!("  {name}");
        }
        *has_printed = true;
    }
}

/// Read [`AnimationClip`]s from the loaded [`Gltf`] and assign them to the
/// entities they control. [`AnimationClip`]s control specific entities, and
/// trying to play them on an [`AnimationPlayer`] controlling a different
/// entities will result in odd animations.
fn setup_animations(
    mut query: Query<(&Name, &mut AnimationPlayer), (With<MorphWeights>, Without<Handle<Mesh>>)>,
    gltf: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut has_setup: Local<bool>,
) {
    if *has_setup {
        return;
    }
    let Some((_, gltf)) = gltf.iter().next() else {
        return;
    };
    // We check compatibility by getting the [`AnimationClip`] out of the
    // [`Assets<AnimationClip>`] and using `compatible_with(&Name)`
    let is_compatible = |name, clip| {
        let Some(clip) = clips.get(clip) else { return false };
        clip.compatible_with(name)
    };
    for (name, mut player) in &mut query {
        let compatible = gltf
            .animations
            .iter()
            .find(|clip| is_compatible(name, clip))
            .unwrap();
        player.play(compatible.clone_weak()).repeat();
        *has_setup = true;
    }
}
