use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(startup_system())
        .run();
}

pub fn startup_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("setup")
        .write_resource::<AssetStorage<Mesh>>()
        .write_resource::<AssetStorage<StandardMaterial>>()
        .build(move |command_buffer, _, (meshes, materials), _| {
            let cube_handle = meshes.add(Mesh::load(MeshType::Cube));
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
                    translation: Translation::new(0.0, 0.0, 0.0),
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
                })
                .build();
        })
}
