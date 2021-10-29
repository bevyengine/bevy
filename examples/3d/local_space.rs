use bevy::math::{const_mat3, const_mat4};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_parent)
        .add_system(move_child)
        .run();
}

#[derive(Component)]
struct ParentMarker {
    pub dir: bool,
}

#[derive(Component)]
struct ChildMarker {
    pub dir: bool,
}

#[allow(clippy::manual_swap)]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Parent
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(shape::Cube { size: 1.0 }.into()),
            material: materials.add(Color::rgb(1.0, 0.2, 0.2).into()),
            ..Default::default()
        })
        .insert(ParentMarker { dir: false })
        .with_children(|c| {
            // Child
            c.spawn_bundle(PbrBundle {
                mesh: meshes.add(shape::Cube { size: 0.5 }.into()),
                material: materials.add(Color::rgb(0.2, 1.0, 1.0).into()),
                ..Default::default()
            })
            .insert(LocalSpace {
                translation: const_mat3!(
                    [0.5, 1.5, 0.0], // You can combine multiple dimensions to rotate the space and values that don't add up to 1.0 scale the space
                    [1.0, 0.0, 0.0], // You can also switch axis, here the parent's y axis corresponds to the child's x axis
                    [0.0, 0.0, 1.0]
                ),
                rotation: const_mat4!(
                    [0.0, -1.0, 0.0, 0.0], // You can also use negative values to flip the direction a transformation applies
                    [1.0, 0.0, 0.0, 0.0], // Again, we are mapping the parent's x and y to the child's y and x
                    [0.0, 0.0, 1.0, 0.0], // Note: this is being applied to a quaternion, which is supposed to be normalized, so some values can affect the scale
                    [0.0, 0.0, 0.0, 1.0]
                ),
                ..Default::default()
            })
            .insert(ChildMarker { dir: false });
        });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

// Parent movement system
// By moving and rotating the parent along the x axis, we can get the child to move and rotate on the y axis (movement is a bit on the x axis as well)
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
        // rotate parent on x axis
        transform.rotate(Quat::from_rotation_x(10.0 * time.delta_seconds()));
    }
}

// Child movement system
// Demonstrates that we can still modify the transform of the child
fn move_child(mut child_query: Query<(&mut Transform, &mut ChildMarker)>, time: Res<Time>) {
    for (mut transform, mut child) in child_query.iter_mut() {
        // just a simple endless cycle of going back and forth
        if child.dir {
            transform.translation.z += 10.0 * time.delta_seconds();
        } else {
            transform.translation.z -= 10.0 * time.delta_seconds();
        }
        // switch direction when out of bounds
        if transform.translation.z >= 10.0 {
            child.dir = false;
        } else if transform.translation.z <= -10.0 {
            child.dir = true;
        }
        // rotate parent on x axis
        transform.rotate(Quat::from_rotation_z(10.0 * time.delta_seconds()));
    }
}
