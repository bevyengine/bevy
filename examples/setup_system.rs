use bevy::prelude::*;

fn main() {
    App::build().add_defaults().add_setup_system(setup_system()).run();
}

pub fn setup_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("setup")
        .write_resource::<AssetStorage<Mesh>>()
        .write_resource::<AssetStorage<StandardMaterial>>()
        .build(move |command_buffer, _, (meshes, materials), _| {
            let cube_handle = meshes.add(Mesh::load(MeshType::Cube));
            let plane_handle = meshes.add(Mesh::load(MeshType::Plane { size: 10.0 }));
            let cube_material_handle = materials.add(StandardMaterial {
                albedo: Color::rgb(0.5, 0.4, 0.3),
                ..Default::default()
            });
            let plane_material_handle = materials.add(StandardMaterial {
                albedo: Color::rgb(0.1, 0.2, 0.1),
                ..Default::default()
            });

            command_buffer
                .build()
                // plane
                .add_entity(MeshEntity {
                    mesh: plane_handle,
                    material: plane_material_handle,
                    // renderable: Renderable::instanced(),
                    ..Default::default()
                })
                // cube
                .add_entity(MeshEntity {
                    mesh: cube_handle,
                    material: cube_material_handle,
                    // renderable: Renderable::instanced(),
                    translation: Translation::new(-1.5, 0.0, 1.0),
                    ..Default::default()
                })
                // cube
                .add_entity(MeshEntity {
                    mesh: cube_handle,
                    material: cube_material_handle,
                    // renderable: Renderable::instanced(),
                    translation: Translation::new(1.5, 0.0, 1.0),
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
