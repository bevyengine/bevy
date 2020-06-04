use bevy::{
    input::keyboard::{KeyboardInput, VirtualKeyCode},
    prelude::*,
};

fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<State>()
        .add_startup_system(setup.system())
        .add_system(collect_input.system())
        .add_system(move_on_input.system())
        .run();
}

#[derive(Default)]
struct State {
    event_reader: EventReader<KeyboardInput>,
    moving_right: bool,
    moving_left: bool,
}

/// adjusts move state based on keyboard input
fn collect_input(mut state: ResMut<State>, keyboard_input_events: Res<Events<KeyboardInput>>) {
    for event in state.event_reader.iter(&keyboard_input_events) {
        match event {
            KeyboardInput {
                virtual_key_code: Some(VirtualKeyCode::Left),
                state: element_state,
                ..
            } => {
                state.moving_left = element_state.is_pressed();
            }
            KeyboardInput {
                virtual_key_code: Some(VirtualKeyCode::Right),
                state: element_state,
                ..
            } => {
                state.moving_right = element_state.is_pressed();
            }
            _ => {}
        }
    }
}

/// moves our cube left when the "left" key is pressed. moves it right when the "right" key is pressed
fn move_on_input(
    state: Res<State>,
    time: Res<Time>,
    mut translation: ComMut<Translation>,
    _: Com<Handle<Mesh>>,
) {
    if state.moving_left {
        translation.0 += math::vec3(1.0, 0.0, 0.0) * time.delta_seconds;
    }

    if state.moving_right {
        translation.0 += math::vec3(-1.0, 0.0, 0.0) * time.delta_seconds;
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
