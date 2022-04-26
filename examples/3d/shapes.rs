use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotate)
        .run();
}

#[derive(Component)]
struct Shape;

const X_EXTENT: f32 = 14.;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: ResMut<AssetServer>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/uv_debug.png")),
        ..default()
    });

    let shapes = [
        meshes.add(shape::Cube::default().into()),
        meshes.add(shape::Box::default().into()),
        meshes.add(shape::Capsule::default().into()),
        meshes.add(shape::Torus::default().into()),
        meshes.add(shape::Icosphere::default().into()),
        meshes.add(shape::UVSphere::default().into()),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands
            .spawn_bundle(PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform {
                    translation: Vec3::new(
                        -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                        2.0,
                        0.0,
                    ),
                    ..default()
                },
                ..Default::default()
            })
            .insert(Shape);
    }

    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 100.,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..Default::default()
    });

    // ground plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(shape::Plane { size: 50. }.into()),
        material: materials.add(Color::SILVER.into()),
        ..Default::default()
    });

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..Default::default()
    });
}

fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.seconds_since_startup() as f32 / 2.)
            * Quat::from_rotation_x(-std::f32::consts::PI / 4.)
    }
}
