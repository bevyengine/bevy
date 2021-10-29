use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_parent)
        .run();
}

#[derive(Component)]
struct ParentMarker {
    pub dir: bool,
}

#[allow(clippy::manual_swap)]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // parent
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(shape::Cube { size: 1.0 }.into()),
            material: materials.add(Color::rgb(1.0, 0.2, 0.2).into()),
            ..Default::default()
        })
        .insert(ParentMarker { dir: false })
        .with_children(|c| {
            // child
            c.spawn_bundle(PbrBundle {
                mesh: meshes.add(
                    shape::Icosphere {
                        radius: 1.0,
                        subdivisions: 2,
                    }
                    .into(),
                ),
                material: materials.add(Color::rgb(0.2, 1.0, 1.0).into()),
                ..Default::default()
            })
            .insert(Relation {
                translation: Some(|v| {
                    // for this example, we are just switching the x and y axis
                    let temp = v.x;
                    v.x = v.y;
                    v.y = temp;
                }),
                ..Default::default()
            });
        });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

// parent movement system
// by moving the parent left and right, the child, whose relation switches the x and y axis, will move up and down
fn move_parent(mut parent_query: Query<(&mut Transform, &mut ParentMarker)>, time: Res<Time>) {
    for (mut transform, mut parent) in parent_query.iter_mut() {
        // just a simple endless cycle of going back and forth
        if parent.dir {
            transform.translation.x += 10.0 * time.delta_seconds();
        } else {
            transform.translation.x -= 10.0 * time.delta_seconds();
        }
        // switch direction when out of bounds
        if transform.translation.x >= 10.0 {
            parent.dir = false;
        } else if transform.translation.x <= -10.0 {
            parent.dir = true;
        }
    }
}
