//! Test that transforming a mesh correctly updates normals and tangents.

use bevy::camera::ScalingMode;
use bevy::core_pipeline::prepass::NormalPrepass;
use bevy::mesh::{SphereKind, SphereMeshBuilder};
use bevy::prelude::*;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_environment, setup_meshes))
        .add_systems(Update, animate_light);

    #[cfg(feature = "bevy_ci_testing")]
    app.add_systems(Startup, ci::setup);

    app.run();
}

fn setup_environment(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let description = r#"(left to right)
  0: Original mesh.
  1: Transformed via mesh attributes.
  2: Transformed via mesh attributes + normals and tangents recalculated.
  3: Transformed via entity.

Controls:
  F1: Cycle render debug overlay"#;

    commands.spawn((
        Text::new(description),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 13.0,
                min_height: 5.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        NormalPrepass,
    ));

    commands.spawn((
        Transform::from_xyz(1.0, 1.0, 0.5).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
    ));

    commands.spawn((
        Transform::from_xyz(0.0, 0.0, -11.0),
        Mesh3d(mesh_assets.add(Plane3d::default().mesh().size(100.0, 100.0).normal(Dir3::Z))),
        MeshMaterial3d(material_assets.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.05, 0.15),
            reflectance: 0.2,
            ..default()
        })),
    ));
}

fn setup_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let material = MeshMaterial3d(material_assets.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.2),
        // Add anisotropy so that lighting is dependent on tangents.
        anisotropy_rotation: 0.5,
        anisotropy_strength: 1.0,
        ..Default::default()
    }));

    let transform = Transform::from_scale(Vec3::new(1.5, 0.5, 1.0)).with_rotation(
        Quat::from_axis_angle(Vec3::splat(1.0).normalize(), 135.0_f32.to_radians()),
    );

    let original_mesh = Mesh::from(SphereMeshBuilder::new(
        1.0,
        SphereKind::Uv {
            sectors: 128,
            stacks: 64,
        },
    ))
    .with_computed_normals()
    .with_generated_tangents()
    .unwrap();

    let transformed_mesh = original_mesh.clone().transformed_by(transform);

    let recalculated_mesh = transformed_mesh
        .clone()
        .with_computed_normals()
        .with_generated_tangents()
        .unwrap();

    let original_mesh = mesh_assets.add(original_mesh);
    let transformed_mesh = mesh_assets.add(transformed_mesh);
    let recalculated_mesh = mesh_assets.add(recalculated_mesh);

    for (mesh_handle, transform) in [
        (&original_mesh, Transform::from_xyz(-4.5, 0.0, -10.0)),
        (&transformed_mesh, Transform::from_xyz(-1.5, 0.0, -10.0)),
        (&recalculated_mesh, Transform::from_xyz(1.5, 0.0, -10.0)),
        (
            &original_mesh,
            Transform::from_xyz(4.5, 0.0, -10.0) * transform,
        ),
    ] {
        commands.spawn((Mesh3d(mesh_handle.clone()), transform, material.clone()));
    }
}

fn animate_light(mut lights: Query<&mut Transform, With<DirectionalLight>>, _time: Res<Time>) {
    #[cfg(feature = "bevy_ci_testing")]
    let x = 0.0;

    #[cfg(not(feature = "bevy_ci_testing"))]
    let x = ops::cos(_time.elapsed_secs());

    for mut transform in lights.iter_mut() {
        transform.translation = vec3(x, 1.0, 1.0);
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}

#[cfg(feature = "bevy_ci_testing")]
mod ci {
    use super::*;
    use bevy::dev_tools::ci_testing::{CiTestingConfig, CiTestingEvent, CiTestingEventOnFrame};

    pub fn setup(mut ci_config: ResMut<CiTestingConfig>) {
        const DELAY: u32 = 100;

        ci_config.events.push(CiTestingEventOnFrame(
            DELAY,
            CiTestingEvent::NamedScreenshot("test_transform_mesh".into()),
        ));

        ci_config
            .events
            .push(CiTestingEventOnFrame(DELAY * 2, CiTestingEvent::AppExit));
    }
}
