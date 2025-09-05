//! Create and play an animation defined by code that operates on the [`Transform`] component.

use std::f32::consts::PI;

use bevy::{
    animation::{animated_field, AnimationTarget, AnimationTargetId},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 150.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 500_000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.5, 0.0),
    ));

    // Let's use the `Name` component to target entities. We can use anything we
    // like, but names are convenient.
    let planet = Name::new("planet");
    let orbit_controller = Name::new("orbit_controller");
    let satellite = Name::new("satellite");

    // Creating the animation
    let mut animation = AnimationClip::default();
    // A curve can modify a single part of a transform: here, the translation.
    let planet_animation_target_id = AnimationTargetId::from_name(&planet);
    animation.add_curve_to_target(
        planet_animation_target_id,
        AnimatableCurve::new(
            animated_field!(Transform::translation),
            UnevenSampleAutoCurve::new([0.0, 1.0, 2.0, 3.0, 4.0].into_iter().zip([
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(-1.0, 0.0, 1.0),
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, -1.0),
                // in case seamless looping is wanted, the last keyframe should
                // be the same as the first one
                Vec3::new(1.0, 0.0, 1.0),
            ]))
            .expect("should be able to build translation curve because we pass in valid samples"),
        ),
    );
    // Or it can modify the rotation of the transform.
    // To find the entity to modify, the hierarchy will be traversed looking for
    // an entity with the right name at each level.
    let orbit_controller_animation_target_id =
        AnimationTargetId::from_names([planet.clone(), orbit_controller.clone()].iter());
    animation.add_curve_to_target(
        orbit_controller_animation_target_id,
        AnimatableCurve::new(
            animated_field!(Transform::rotation),
            UnevenSampleAutoCurve::new([0.0, 1.0, 2.0, 3.0, 4.0].into_iter().zip([
                Quat::IDENTITY,
                Quat::from_axis_angle(Vec3::Y, PI / 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
                Quat::IDENTITY,
            ]))
            .expect("Failed to build rotation curve"),
        ),
    );
    // If a curve in an animation is shorter than the other, it will not repeat
    // until all other curves are finished. In that case, another animation should
    // be created for each part that would have a different duration / period.
    let satellite_animation_target_id = AnimationTargetId::from_names(
        [planet.clone(), orbit_controller.clone(), satellite.clone()].iter(),
    );
    animation.add_curve_to_target(
        satellite_animation_target_id,
        AnimatableCurve::new(
            animated_field!(Transform::scale),
            UnevenSampleAutoCurve::new(
                [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0]
                    .into_iter()
                    .zip([
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
            )
            .expect("Failed to build scale curve"),
        ),
    );
    // There can be more than one curve targeting the same entity path.
    animation.add_curve_to_target(
        AnimationTargetId::from_names(
            [planet.clone(), orbit_controller.clone(), satellite.clone()].iter(),
        ),
        AnimatableCurve::new(
            animated_field!(Transform::rotation),
            UnevenSampleAutoCurve::new([0.0, 1.0, 2.0, 3.0, 4.0].into_iter().zip([
                Quat::IDENTITY,
                Quat::from_axis_angle(Vec3::Y, PI / 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
                Quat::IDENTITY,
            ]))
            .expect("should be able to build translation curve because we pass in valid samples"),
        ),
    );

    // Create the animation graph
    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));

    // Create the animation player, and set it to repeat
    let mut player = AnimationPlayer::default();
    player.play(animation_index).repeat();

    // Create the scene that will be animated
    // First entity is the planet
    let planet_entity = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::default())),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
            // Add the animation graph and player
            planet,
            AnimationGraphHandle(graphs.add(graph)),
            player,
        ))
        .id();
    commands.entity(planet_entity).insert((
        AnimationTarget {
            id: planet_animation_target_id,
            player: planet_entity,
        },
        children![(
            Transform::default(),
            Visibility::default(),
            orbit_controller,
            AnimationTarget {
                id: orbit_controller_animation_target_id,
                player: planet_entity,
            },
            children![(
                Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
                MeshMaterial3d(materials.add(Color::srgb(0.3, 0.9, 0.3))),
                Transform::from_xyz(1.5, 0.0, 0.0),
                AnimationTarget {
                    id: satellite_animation_target_id,
                    player: planet_entity,
                },
                satellite,
            )],
        )],
    ));
}
