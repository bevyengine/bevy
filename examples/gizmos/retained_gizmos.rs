//! This example demonstrates Bevy's visual debugging using retained gizmos.

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::*,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        //.init_gizmo_group::<MyRoundGizmos>()
        .add_systems(Startup, setup)
        // .add_systems(
        //     Update,
        //     (
        //         draw_example_collection,
        //         update_config,
        //         update_retained_gizmo_visibility,
        //     ),
        // )
        .run();
}

fn setup(mut commands: Commands, mut gizmo_assets: ResMut<Assets<GizmoAsset>>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1.5, 6.).looking_at(Vec3::NEG_Z, Vec3::Y),
        FreeCamera::default(),
    ));

    let mut gizmo = GizmoAsset::new();

    // When drawing a lot of static lines a Gizmo component can have
    // far better performance than the Gizmos system parameter,
    // but the system parameter will perform better for smaller lines that update often.

    // we'll sprinkle spheres made of 3,000 lines throughout the scene
    // and make the blink
    gizmo
        .sphere(Isometry3d::IDENTITY, 0.5, LIGHT_GOLDENROD_YELLOW)
        .resolution(3_000 / 3);

    commands.spawn((
        Gizmo {
            handle: gizmo_assets.add(gizmo),
            line_config: GizmoLineConfig {
                width: 2.,
                ..default()
            },
            ..default()
        },
        Transform::IDENTITY,
    ));
}
