//! Interactive transform gizmo example.
//!
//! Demonstrates translate, rotate, and scale gizmos with click-to-select.
//! - Click an object to select it
//! - **1** = Translate, **2** = Rotate, **3** = Scale
//! - **X** = Toggle world/local space

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    gizmos::transform_gizmo::{
        TransformGizmoCamera, TransformGizmoFocus, TransformGizmoMode, TransformGizmoPlugin,
        TransformGizmoSpace,
    },
    picking::Pickable,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FreeCameraPlugin,
            MeshPickingPlugin,
            TransformGizmoPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (gizmo_mode_keys, update_instructions))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Instructions
    commands.spawn((
        Text::new(
            "Click an object to select it\n1: Translate | 2: Rotate | 3: Scale | X: Toggle space",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        InstructionsText,
    ));

    // Ground plane (not pickable)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.3, 0.3))),
        Pickable::IGNORE,
    ));

    // Cube (starts selected)
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.3, 0.3))),
            Transform::from_xyz(-2.0, 0.5, 0.0),
            TransformGizmoFocus,
        ))
        .observe(on_click_select);

    // Sphere
    commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(0.5).mesh().uv(32, 18))),
            MeshMaterial3d(materials.add(Color::srgb(0.3, 0.8, 0.3))),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .observe(on_click_select);

    // Cylinder
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.4, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.3, 0.3, 0.8))),
            Transform::from_xyz(2.0, 0.5, 0.0),
        ))
        .observe(on_click_select);

    // Light
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera::default(),
        TransformGizmoCamera,
    ));
}

fn on_click_select(
    click: On<Pointer<Click>>,
    mut commands: Commands,
    existing: Query<Entity, With<TransformGizmoFocus>>,
) {
    // Remove focus from all entities
    for e in &existing {
        commands.entity(e).remove::<TransformGizmoFocus>();
    }
    // Add focus to clicked entity
    commands.entity(click.entity).insert(TransformGizmoFocus);
}

fn gizmo_mode_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<TransformGizmoMode>,
    mut space: ResMut<TransformGizmoSpace>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        *mode = TransformGizmoMode::Translate;
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        *mode = TransformGizmoMode::Rotate;
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        *mode = TransformGizmoMode::Scale;
    }
    if keyboard.just_pressed(KeyCode::KeyX) {
        *space = match *space {
            TransformGizmoSpace::World => TransformGizmoSpace::Local,
            TransformGizmoSpace::Local => TransformGizmoSpace::World,
        };
    }
}

#[derive(Component)]
struct InstructionsText;

fn update_instructions(
    mode: Res<TransformGizmoMode>,
    space: Res<TransformGizmoSpace>,
    mut text: Single<&mut Text, With<InstructionsText>>,
) {
    let mode_str = match *mode {
        TransformGizmoMode::Translate => "Translate",
        TransformGizmoMode::Rotate => "Rotate",
        TransformGizmoMode::Scale => "Scale",
    };
    let space_str = match *space {
        TransformGizmoSpace::World => "World",
        TransformGizmoSpace::Local => "Local",
    };
    text.0 = format!(
        "Click an object to select it\n1: Translate | 2: Rotate | 3: Scale | X: Toggle space\nMode: {mode_str} | Space: {space_str}"
    );
}
