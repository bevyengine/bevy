use bevy::{
    input::keyboard::{KeyboardInput, VirtualKeyCode},
    prelude::*,
};

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<State>()
        .add_startup_system(setup.system())
        .add_system(move_on_input.system())
        .run();
}

#[derive(Default)]
struct State {
    event_reader: EventReader<KeyboardInput>,
}

/// moves our cube left when the "left" key is pressed. moves it right when the "right" key is pressed
fn move_on_input(
    world: &mut SubWorld,
    mut state: ResMut<State>,
    time: Res<Time>,
    keyboard_input_events: Res<Events<KeyboardInput>>,
    query: &mut Query<(Write<Translation>, Read<Handle<Mesh>>)>,
) {
    let mut moving_left = false;
    let mut moving_right = false;
    for event in state.event_reader.iter(&keyboard_input_events) {
        if let KeyboardInput {
            virtual_key_code: Some(key_code),
            state,
            ..
        } = event
        {
            if *key_code == VirtualKeyCode::Left {
                moving_left = state.is_pressed();
            } else if *key_code == VirtualKeyCode::Right {
                moving_right = state.is_pressed();
            }
        }
    }

    for (mut translation, _) in query.iter_mut(world) {
        if moving_left {
            translation.0 += math::vec3(1.0, 0.0, 0.0) * time.delta_seconds;
        }

        if moving_right {
            translation.0 += math::vec3(-1.0, 0.0, 0.0) * time.delta_seconds;
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
