//! This example demonstrates how to use aabb and frustum gizmos for frustum culling debugging.
//! 
//! Aabb gizmos are used to visualize the [`Aabb`](bevy::camera::primitives::Aabb) of an entity.
//! Frustum gizmos are used to visualize the [`Frustum`](bevy::camera::primitives::Frustum) of a camera.
//! Both can be used together to visualize frustum culling for a given camera.
//! 
//! This example shows a scene with an inactive camera that has its [`Frustum`](bevy::camera::primitives::Frustum)
//! gizmo visible. A rotating collection of shapes, with their individual
//! [`Aabb`](bevy::camera::primitives::Aabb) gizmos visible, circle in and out of this
//! camera's frustum. The [`Aabb`](bevy::camera::primitives::Aabb) gizmos are red
//! when they have been culled. They change to green when the shape
//! is considered visible by this camera and would be extracted for rendering.
//! A second active camera, controllable via the FreeCameraPlugin, is used to observe the scene.

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    camera::visibility::{NoCpuCulling, VisibilitySystems},
    gizmos::aabb::ShowAabbGizmo,
    prelude::*,
};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_shapes, update_config))
        .add_systems(PostUpdate, detect_visibility_of_shapes
            .after(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible))
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
struct Shape;

const SHAPE_MOVEMENT_RADIUS: f32 = 10.0;

fn setup(
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    // The camera that the user controls to observe the scene.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-5., 1.5, 6.).looking_at(Vec3::new(7., 1.5, 0.), Vec3::Y),
        FreeCamera::default(),
        // This ensures that this camera does not interfere with debugging frustum culling.
        // Culling logic will only be applied using the other camera's frustum.
        NoCpuCulling,
    ));

    // The camera that we want to debug frustum culling for.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1.5, 0.).looking_at(Vec3::new(1.0, 1.5, 0.), Vec3::Y),
        // To visualize its frustum, we add the [`ShowFrustumGizmo`] component.
        ShowFrustumGizmo::default(),
        Camera {
            // is_active is temporarily set to false so that what it sees
            // is not rendered to the screen.
            is_active: false,
            ..Default::default()
        },
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(SHAPE_MOVEMENT_RADIUS * 2., SHAPE_MOVEMENT_RADIUS * 2.))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // Light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Example instructions
    commands.spawn((
        Text::new(
            "Press 'T' to toggle drawing gizmos on top of everything else in the scene\n\
            Press 'P' to toggle perspective for line gizmos\n\
            Hold 'Left' or 'Right' to change the line width of straight gizmos\n\
            Hold 'Up' or 'Down' to change the line width of round gizmos\n\
            Press '1' or '2' to toggle the visibility of straight gizmos or round gizmos\n\
            Press 'B' to show all AABB boxes\n\
            Press 'U' or 'I' to cycle through line styles for straight or round gizmos\n\
            Press 'J' or 'K' to cycle through line joins for straight or round gizmos\n\
            Press 'Spacebar' to toggle pause",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    // Configure AABB's to be drawn and have a default color of red
    let (_, aabb_gizmo_config) = config_store.config_mut::<AabbGizmoConfigGroup>();
    aabb_gizmo_config.draw_all = true;
    aabb_gizmo_config.default_color = Some(Color::LinearRgba(LinearRgba::RED));

    // Configure shapes that will have their AABB's drawn
    let white_matl = materials.add(Color::WHITE);
    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
    ];
    for (i, shape) in shapes.into_iter().enumerate() {
        let shape_angle = i as f32 * 2. * PI / 5.;
        let (s, c) = ops::sin_cos(shape_angle);
        let (x, z) = (SHAPE_MOVEMENT_RADIUS * c, SHAPE_MOVEMENT_RADIUS * s);
        commands
            .spawn((
                Mesh3d(shape),
                MeshMaterial3d(white_matl.clone()),
                Transform::from_xyz(
                    x,
                    1.5,
                    z,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
                // This component is added here so that we can override the color later.
                ShowAabbGizmo::default()
            ));
    }
}

// A system that rotates shapes in place and also moves them in a circle around the camera
fn move_shapes(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Shape>>,
) {
    let dt = time.delta_secs();
    for mut transform in &mut query {
        transform.rotate_y(dt / 2.);
    }
}

// A system that changes the color of the [`AabbGizmo`](bevy::gizmos::Aabb)
// if they are considered visible by the camera.
fn detect_visibility_of_shapes(
    mut query: Query<(&ViewVisibility, &mut ShowAabbGizmo), With<Shape>>,
) {
    for (visibility, mut gizmo) in query.iter_mut() {
        if visibility.get() {
            gizmo.color = Some(Color::LinearRgba(LinearRgba::GREEN));
        } else {
            gizmo.color = None;
        }
    }
}

fn update_config(
    mut config_store: ResMut<GizmoConfigStore>,
    keyboard: Res<ButtonInput<KeyCode>>,
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if keyboard.just_pressed(KeyCode::KeyT) {
        for (_, config, _) in config_store.iter_mut() {
            config.depth_bias = if config.depth_bias == 0. { -1. } else { 0. };
        }
    }
    if keyboard.just_pressed(KeyCode::KeyP) {
        for (_, config, _) in config_store.iter_mut() {
            // Toggle line perspective
            config.line.perspective ^= true;
            // Increase the line width when line perspective is on
            config.line.width *= if config.line.perspective { 5. } else { 1. / 5. };
        }
    }

    let (config, _) = config_store.config_mut::<FrustumGizmoConfigGroup>();
    if keyboard.pressed(KeyCode::ArrowRight) {
        config.line.width += 5. * real_time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        config.line.width -= 5. * real_time.delta_secs();
        config.line.width = config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyU) {
        config.line.style = match config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyJ) {
        config.line.joints = match config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    let (my_config, _) = config_store.config_mut::<AabbGizmoConfigGroup>();
    if keyboard.pressed(KeyCode::ArrowUp) {
        my_config.line.width += 5. * real_time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        my_config.line.width -= 5. * real_time.delta_secs();
        my_config.line.width = my_config.line.width.clamp(0., 50.);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        my_config.enabled ^= true;
    }
    if keyboard.just_pressed(KeyCode::KeyI) {
        my_config.line.style = match my_config.line.style {
            GizmoLineStyle::Solid => GizmoLineStyle::Dotted,
            GizmoLineStyle::Dotted => GizmoLineStyle::Dashed {
                gap_scale: 3.0,
                line_scale: 5.0,
            },
            _ => GizmoLineStyle::Solid,
        };
    }
    if keyboard.just_pressed(KeyCode::KeyK) {
        my_config.line.joints = match my_config.line.joints {
            GizmoLineJoint::Bevel => GizmoLineJoint::Miter,
            GizmoLineJoint::Miter => GizmoLineJoint::Round(4),
            GizmoLineJoint::Round(_) => GizmoLineJoint::None,
            GizmoLineJoint::None => GizmoLineJoint::Bevel,
        };
    }

    if keyboard.just_pressed(KeyCode::KeyB) {
        // AABB gizmos are normally only drawn on entities with a ShowAabbGizmo component
        // We can change this behavior in the configuration of AabbGizmoGroup
        config_store.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        virtual_time.toggle();
    }
}
