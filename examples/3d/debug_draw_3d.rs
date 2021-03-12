use bevy::{debug_draw::debug_draw_3d::*, prelude::*};

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        //Add the plugin for DebugDraw
        .add_plugin(DebugDrawPlugin)
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .add_system(deubg_draw_sample_system.system())
        //uncomment this prebuilt system to draw a coordinate gizmo for any entity with a `GlobalTransform`
        // .add_system(debug_draw_all_gizmos.system())
        .run();
}

/// This system takes any entities with GlobalTransform and Rotator
/// and draws a line from it to the origin of the world.
fn deubg_draw_sample_system(
    mut debug_draw: ResMut<DebugDraw3D>,
    query: Query<&GlobalTransform, With<Rotator>>,
) {
    for transform in query.iter() {
        debug_draw.draw_line(Vec3::ZERO, transform.translation, Color::RED);
    }
    let bl = Vec3::new(-2.0, 0.0, -2.0);
    let br = Vec3::new(2.0, 0.0, -2.0);
    let tr = Vec3::new(2.0, 0.0, 2.0);
    let tl = Vec3::new(-2.0, 0.0, 2.0);
    //Draw a square
    debug_draw.draw_line(bl, br, Color::BLUE);
    debug_draw.draw_line(br, tr, Color::BLUE);
    debug_draw.draw_line(tr, tl, Color::BLUE);
    debug_draw.draw_line(tl, bl, Color::BLUE);
}

/// this component indicates what entities should rotate
struct Rotator;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotator>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *=
            Quat::from_axis_angle(Vec3::ONE.normalize_or_zero(), 2.0 * time.delta_seconds());
    }
}

/// set up a 3D scene for testing
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));

    commands
        // parent cube
        .spawn(PbrBundle {
            mesh: cube_handle.clone(),
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|parent| {
            // child cubes
            parent
                .spawn(PbrBundle {
                    mesh: cube_handle.clone(),
                    material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
                    transform: Transform::from_xyz(4.0, 0.0, 0.0),
                    ..Default::default()
                })
                .with(Rotator)
                .spawn(PbrBundle {
                    mesh: cube_handle.clone(),
                    material: materials.add(Color::rgb(0.5, 1.0, 0.5).into()),
                    transform: Transform::from_xyz(0.0, 4.0, 0.0),
                    ..Default::default()
                })
                .with(Rotator)
                .spawn(PbrBundle {
                    mesh: cube_handle.clone(),
                    material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                    transform: Transform::from_xyz(0.0, 0.0, 4.0),
                    ..Default::default()
                })
                .with(Rotator);
        })
        // light
        .spawn(LightBundle {
            transform: Transform::from_xyz(5.0, 2.0, -1.0),
            ..Default::default()
        })
        // camera
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-7.0, 6.0, 4.0)
                .looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
            ..Default::default()
        });
}
