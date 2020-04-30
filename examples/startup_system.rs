use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(startup_system.system())
        .run();
}

/// Set up a simple scene using a "startup system".
/// Startup systems are run exactly once when the app starts up.
/// They run right before "normal" systems run.
fn startup_system(
    command_buffer: &mut CommandBuffer,
    mut meshes: ResourceMut<AssetStorage<Mesh>>,
    mut materials: ResourceMut<AssetStorage<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube));
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
        });
}