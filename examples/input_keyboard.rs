use bevy::{
    input::keyboard::{KeyboardInput, VirtualKeyCode},
    prelude::*,
};

fn main() {
    App::build()
        .add_default_plugins()
        .build_system(move_on_input_system)
        .setup(setup)
        .run();
}

/// moves our cube left when the "left" key is pressed. moves it right when the "right" key is pressed
pub fn move_on_input_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut keyboard_input_event_reader = resources.get_event_reader::<KeyboardInput>();
    let mut moving_left = false;
    let mut moving_right = false;
    SystemBuilder::new("input_handler")
        .read_resource::<Time>()
        .read_resource::<Events<KeyboardInput>>()
        .with_query(<(Write<Translation>, Read<Handle<Mesh>>)>::query())
        .build(
            move |_command_buffer, world, (time, keyboard_input_events), mesh_query| {
                for event in keyboard_input_events.iter(&mut keyboard_input_event_reader) {
                    match event {
                        KeyboardInput {
                            virtual_key_code: Some(VirtualKeyCode::Left),
                            state,
                            ..
                        } => {
                            moving_left = state.is_pressed();
                        }
                        KeyboardInput {
                            virtual_key_code: Some(VirtualKeyCode::Right),
                            state,
                            ..
                        } => {
                            moving_right = state.is_pressed();
                        }
                        _ => {}
                    }
                }

                for (mut translation, _mesh) in mesh_query.iter_mut(world) {
                    if moving_left {
                        translation.0 += math::vec3(1.0, 0.0, 0.0) * time.delta_seconds;
                    }

                    if moving_right {
                        translation.0 += math::vec3(-1.0, 0.0, 0.0) * time.delta_seconds;
                    }
                }
            },
        )
}

/// creates a simple scene
fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));
    let cube_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    world
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
        .add_entity(CameraEntity {
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
