//! Controls morph targets in a loaded scene.
//!
//! Illustrates:
//!!
//! - How to access and modify individual morph target weights.
//!   See the [`update_weights`] system for details.
//! - How to read morph target names in [`name_morphs`].
//! - How to play morph target animations in [`setup_animations`].
use std::f32::consts::PI;

use bevy::{
    gltf::{Gltf, MorphTargetNames},
    prelude::*,
};
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
        .add_systems(Update, (name_morphs, setup_animations, update_weights))
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

/// Marker for weights that are not animated by the animation system.
#[derive(Component)]
struct UpdateWeights;

/// To update weights, query for [`MorphWeights`].
///
/// Note that direct children with a [`Handle<Mesh>`] component of entities
/// with a [`MorphWeights`] component will inherit their parent's weights.
fn update_weights(mut morphs: Query<&mut MorphWeights, With<UpdateWeights>>, time: Res<Time>) {
    let mut t = time.elapsed_seconds();
    let offset_per_weight = PI / 4.0;
    for mut morph in &mut morphs {
        for weight in morph.weights_mut() {
            *weight = t.cos().abs();
            t += offset_per_weight;
        }
    }
}

/// You can get the target names in the `MorphTargetNames` component.
/// They are in the order of the weights.
fn name_morphs(query: Query<(&Name, &MorphTargetNames)>, mut has_printed: Local<bool>) {
    if *has_printed {
        return;
    }
    for (name, target_names) in &query {
        info!("Node {name} has the following targets:");
        for name in &target_names.target_names {
            info!("\t{name}");
        }
        *has_printed = true;
    }
}

/// Read [`AnimationClip`]s from the loaded [`Gltf`] and assign them to the
/// entities they control. [`AnimationClip`]s control specific entities, and
/// trying to play them on an [`AnimationPlayer`] controlling a different
/// entities will result in odd animations.
fn setup_animations(
    mut query: Query<
        (&Name, Entity, Option<&mut AnimationPlayer>),
        (With<MorphWeights>, Without<Handle<Mesh>>),
    >,
    gltf: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut commands: Commands,
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
    for (name, entity, player) in &mut query {
        match player {
            Some(mut player) => {
                let compatible = gltf
                    .animations
                    .iter()
                    .find(|clip| is_compatible(name, clip))
                    .unwrap();
                player.play(compatible.clone_weak()).repeat();
            }
            None => {
                commands.entity(entity).insert(UpdateWeights);
            }
        }
        *has_setup = true;
    }
}
