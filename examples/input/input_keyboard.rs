use bevy::{input::keyboard::KeyCode, prelude::*};
use bevy_input::Input;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(move_on_input.system())
        .run();
}

/// moves our cube left when the "left" key is pressed. moves it right when the "right" key is pressed
fn move_on_input(
    world: &mut SubWorld,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    query: &mut Query<(Write<Translation>, Read<Handle<Mesh>>)>,
) {
    let moving_left = keyboard_input.pressed(KeyCode::Left);
    let moving_right = keyboard_input.pressed(KeyCode::Right);

    if keyboard_input.just_pressed(KeyCode::Left) {
        println!("left just pressed");
    }

    if keyboard_input.just_released(KeyCode::Left) {
        println!("left just released");
    }

    let speed = 3.0;
    for (mut translation, _) in query.iter_mut(world) {
        if moving_left {
            translation.0 += math::vec3(speed, 0.0, 0.0) * time.delta_seconds;
        }

        if moving_right {
            translation.0 += math::vec3(-speed, 0.0, 0.0) * time.delta_seconds;
        }
    }
}

/// creates a simple scene
fn setup(
    command_buffer: &mut CommandBuffer,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    command_buffer
        .build()
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: cube_material_handle,
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // camera
        .add_entity(PerspectiveCameraEntity {
            local_to_world: LocalToWorld::new_sync_disabled(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
