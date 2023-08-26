//! Create and play an animation defined by code that operates on the [`Transform`] component.

use std::f32::consts::PI;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0,
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
) {
    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // The animation API uses the `Name` component to target entities
    let planet = Name::new("planet");
    let orbit_controller = Name::new("orbit_controller");
    let satellite = Name::new("satellite");

    // Creating the animation
    let mut animation = AnimationClip::default();
    // A curve can modify a single part of a transform, here the translation
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            keyframes: Keyframes::Translation(vec![
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(-1.0, 0.0, 1.0),
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, -1.0),
                // in case seamless looping is wanted, the last keyframe should
                // be the same as the first one
                Vec3::new(1.0, 0.0, 1.0),
            ]),
        },
    );
    // Or it can modify the rotation of the transform.
    // To find the entity to modify, the hierarchy will be traversed looking for
    // an entity with the right name at each level
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone(), orbit_controller.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            keyframes: Keyframes::Rotation(vec![
                Quat::IDENTITY,
                Quat::from_axis_angle(Vec3::Y, PI / 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
                Quat::IDENTITY,
            ]),
        },
    );
    // If a curve in an animation is shorter than the other, it will not repeat
    // until all other curves are finished. In that case, another animation should
    // be created for each part that would have a different duration / period
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone(), orbit_controller.clone(), satellite.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0],
            keyframes: Keyframes::Scale(vec![
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
            ]),
        },
    );
    // There can be more than one curve targeting the same entity path
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone(), orbit_controller.clone(), satellite.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            keyframes: Keyframes::Rotation(vec![
                Quat::IDENTITY,
                Quat::from_axis_angle(Vec3::Y, PI / 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
                Quat::IDENTITY,
            ]),
        },
    );

    // Create the animation player, and set it to repeat
    let mut player = AnimationPlayer::default();
    player.play(animations.add(animation)).repeat();

    // Create the scene that will be animated
    // First entity is the planet
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::try_from(shape::Icosphere::default()).unwrap()),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                ..default()
            },
            // Add the Name component, and the animation player
            planet,
            player,
        ))
        .with_children(|p| {
            // This entity is just used for animation, but doesn't display anything
            p.spawn((
                SpatialBundle::INHERITED_IDENTITY,
                // Add the Name component
                orbit_controller,
            ))
            .with_children(|p| {
                // The satellite, placed at a distance of the planet
                p.spawn((
                    PbrBundle {
                        transform: Transform::from_xyz(1.5, 0.0, 0.0),
                        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                        material: materials.add(Color::rgb(0.3, 0.9, 0.3).into()),
                        ..default()
                    },
                    // Add the Name component
                    satellite,
                ));
            });
        });
}
