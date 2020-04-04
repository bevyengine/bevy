use bevy::{prelude::*, plugin::AppPlugin};

#[derive(RegisterAppPlugin)]
pub struct ExamplePlugin;

impl AppPlugin for ExamplePlugin {
    fn build(&self, app_builder: AppBuilder) -> AppBuilder {
        app_builder.setup(setup)
    }

    fn name(&self) -> &str {
        "example"
    }
}

pub fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));
    let plane_handle = mesh_storage.add(Mesh::load(MeshType::Plane { size: 10.0 }));

    world.build()
        // plane
        .add_entity(MeshEntity {
            mesh: plane_handle,
            material: StandardMaterial {
                albedo: Color::rgb(0.1, 0.2, 0.1),
            },
            ..Default::default()
        })
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: StandardMaterial {
                albedo: Color::rgb(0.5, 0.4, 0.3),
            },
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            light: Light {
                color: Color::rgb(0.8, 0.8, 0.5),
                fov: f32::to_radians(60.0),
                depth: 0.1..50.0,
            },
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
}