//! This example showcases a 3D first-person camera.
//!
//! The setup presented here is a very common way of organizing a first-person game
//! where the player can see their own arms. We use two industry terms to differentiate
//! the kinds of models we have:
//!
//! - The *view model* is the model that represents the player's body.
//! - The *world model* is everything else.
//!
//! ## Motivation
//!
//! The reason for this distinction is that these two models should be rendered with different field of views (FOV).
//! The view model is typically designed and animated with a very specific FOV in mind, so it is
//! generally *fixed* and cannot be changed by a player. The world model, on the other hand, should
//! be able to change its FOV to accommodate the player's preferences for the following reasons:
//! - *Accessibility*: How prone is the player to motion sickness? A wider FOV can help.
//! - *Tactical preference*: Does the player want to see more of the battlefield?
//!   Or have a more zoomed-in view for precision aiming?
//! - *Physical considerations*: How well does the in-game FOV match the player's real-world FOV?
//!   Are they sitting in front of a monitor or playing on a TV in the living room? How big is the screen?
//!
//! ## Implementation
//!
//! The `Player` is an entity holding two cameras, one for each model. The view model camera has a fixed
//! FOV of 70 degrees, while the world model camera has a variable FOV that can be changed by the player.
//!
//! We use different `RenderLayers` to select what to render.
//!
//! - The world model camera has no explicit `RenderLayers` component, so it uses the layer 0.
//!   All static objects in the scene are also on layer 0 for the same reason.
//! - The view model camera has a `RenderLayers` component with layer 1, so it only renders objects
//!   explicitly assigned to layer 1. The arm of the player is one such object.
//!   The order of the view model camera is additionally bumped to 1 to ensure it renders on top of the world model.
//! - The light source in the scene must illuminate both the view model and the world model, so it is
//!   assigned to both layers 0 and 1.
//!
//! ## Controls
//!
//! | Key Binding          | Action        |
//! |:---------------------|:--------------|
//! | mouse                | Look around   |
//! | arrow up             | Decrease FOV  |
//! | arrow down           | Increase FOV  |

use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::visibility::RenderLayers, color::palettes::tailwind,
    input::mouse::AccumulatedMouseMotion, light::NotShadowCaster, prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (
                spawn_view_model,
                spawn_world_model,
                spawn_lights,
                spawn_text,
            ),
        )
        .add_systems(Update, (move_player, change_fov))
        .run();
}

#[derive(Debug, Component)]
struct Player;

#[derive(Debug, Component, Deref, DerefMut)]
struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(
            // These factors are just arbitrary mouse sensitivity values.
            // It's often nicer to have a faster horizontal sensitivity than vertical.
            // We use a component for them so that we can make them user-configurable at runtime
            // for accessibility reasons.
            // It also allows you to inspect them in an editor if you `Reflect` the component.
            Vec2::new(0.003, 0.002),
        )
    }
}

#[derive(Debug, Component)]
struct WorldModelCamera;

/// Used implicitly by all entities without a `RenderLayers` component.
/// Our world model camera and all objects other than the player are on this layer.
/// The light source belongs to both layers.
const DEFAULT_RENDER_LAYER: usize = 0;

/// Used by the view model camera and the player's arm.
/// The light source belongs to both layers.
const VIEW_MODEL_RENDER_LAYER: usize = 1;

fn spawn_view_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let arm = meshes.add(Cuboid::new(0.1, 0.1, 0.5));
    let arm_material = materials.add(Color::from(tailwind::TEAL_200));

    commands.spawn((
        Player,
        CameraSensitivity::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Visibility::default(),
        children![
            (
                WorldModelCamera,
                Camera3d::default(),
                Projection::from(PerspectiveProjection {
                    fov: 90.0_f32.to_radians(),
                    ..default()
                }),
            ),
            // Spawn view model camera.
            (
                Camera3d::default(),
                Camera {
                    // Bump the order to render on top of the world model.
                    order: 1,
                    ..default()
                },
                Projection::from(PerspectiveProjection {
                    fov: 70.0_f32.to_radians(),
                    ..default()
                }),
                // Only render objects belonging to the view model.
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            ),
            // Spawn the player's right arm.
            (
                Mesh3d(arm),
                MeshMaterial3d(arm_material),
                Transform::from_xyz(0.2, -0.1, -0.25),
                // Ensure the arm is only rendered by the view model camera.
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                // The arm is free-floating, so shadows would look weird.
                NotShadowCaster,
            ),
        ],
    ));
}

fn spawn_world_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)));
    let cube = meshes.add(Cuboid::new(2.0, 0.5, 1.0));
    let material = materials.add(Color::WHITE);

    // The world model camera will render the floor and the cubes spawned in this system.
    // Assigning no `RenderLayers` component defaults to layer 0.

    commands.spawn((Mesh3d(floor), MeshMaterial3d(material.clone())));

    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(0.0, 0.25, -3.0),
    ));

    commands.spawn((
        Mesh3d(cube),
        MeshMaterial3d(material),
        Transform::from_xyz(0.75, 1.75, 0.0),
    ));
}

fn spawn_lights(mut commands: Commands) {
    commands.spawn((
        PointLight {
            color: Color::from(tailwind::ROSE_300),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-2.0, 4.0, -0.75),
        // The light source illuminates both the world model and the view model.
        RenderLayers::from_layers(&[DEFAULT_RENDER_LAYER, VIEW_MODEL_RENDER_LAYER]),
    ));
}

fn spawn_text(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        })
        .with_child(Text::new(concat!(
            "Move the camera with your mouse.\n",
            "Press arrow up to decrease the FOV of the world model.\n",
            "Press arrow down to increase the FOV of the world model."
        )));
}

fn move_player(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    player: Single<(&mut Transform, &CameraSensitivity), With<Player>>,
) {
    let (mut transform, camera_sensitivity) = player.into_inner();

    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        // Note that we are not multiplying by delta_time here.
        // The reason is that for mouse movement, we already get the full movement that happened since the last frame.
        // This means that if we multiply by delta_time, we will get a smaller rotation than intended by the user.
        // This situation is reversed when reading e.g. analog input from a gamepad however, where the same rules
        // as for keyboard input apply. Such an input should be multiplied by delta_time to get the intended rotation
        // independent of the framerate.
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        // If the pitch was ±¹⁄₂ π, the camera would look straight up or down.
        // When the user wants to move the camera back to the horizon, which way should the camera face?
        // The camera has no way of knowing what direction was "forward" before landing in that extreme position,
        // so the direction picked will for all intents and purposes be arbitrary.
        // Another issue is that for mathematical reasons, the yaw will effectively be flipped when the pitch is at the extremes.
        // To not run into these issues, we clamp the pitch to a safe range.
        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

fn change_fov(
    input: Res<ButtonInput<KeyCode>>,
    mut world_model_projection: Single<&mut Projection, With<WorldModelCamera>>,
) {
    let Projection::Perspective(perspective) = world_model_projection.as_mut() else {
        unreachable!(
            "The `Projection` component was explicitly built with `Projection::Perspective`"
        );
    };

    if input.pressed(KeyCode::ArrowUp) {
        perspective.fov -= 1.0_f32.to_radians();
        perspective.fov = perspective.fov.max(20.0_f32.to_radians());
    }
    if input.pressed(KeyCode::ArrowDown) {
        perspective.fov += 1.0_f32.to_radians();
        perspective.fov = perspective.fov.min(160.0_f32.to_radians());
    }
}
