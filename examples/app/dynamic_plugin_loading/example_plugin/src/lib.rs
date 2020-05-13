use bevy::prelude::*;

#[derive(DynamicAppPlugin)]
pub struct ExamplePlugin;

impl AppPlugin for ExamplePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup);
    }
}

pub fn setup(world: &mut World, resources: &mut Resources) {
    let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();
    let mut materials = resources
        .get_mut::<Assets<StandardMaterial>>()
        .unwrap();
    let cube_handle = meshes.add(Mesh::from(shape::Cube));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    world
        .build()
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: cube_material_handle,
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
