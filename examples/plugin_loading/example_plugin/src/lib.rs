use bevy::{prelude::*, plugin::AppPlugin};
use bevy_derive::RegisterAppPlugin;

#[derive(RegisterAppPlugin)]
pub struct ExamplePlugin;

impl AppPlugin for ExamplePlugin {
    fn build(&self, app_builder: AppBuilder) -> AppBuilder {
        app_builder.setup_world(setup)
    }

    fn name(&self) -> &'static str {
        "example"
    }
}

pub fn setup(world: &mut World, resources: &mut Resources) {
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane { size: 10.0 });

    let (cube_handle, plane_handle) = {
        let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        (mesh_storage.add(cube), mesh_storage.add(plane))
    };

    world.build()
        // plane
        .add_archetype(MeshEntity {
            mesh: plane_handle,
            material: StandardMaterial {
                albedo: math::vec4(0.1, 0.2, 0.1, 1.0).into(),
            },
            ..Default::default()
        })
        // cube
        .add_archetype(MeshEntity {
            mesh: cube_handle,
            material: StandardMaterial {
                albedo: math::vec4(0.5, 0.4, 0.3, 1.0).into(),
            },
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        // .add_archetype(LightEntity {
        //     light: Light {
        //         color: wgpu::Color {
        //             r: 0.8,
        //             g: 0.8,
        //             b: 0.5,
        //             a: 1.0,
        //         },
        //         fov: f32::to_radians(60.0),
        //         depth: 0.1..50.0,
        //         target_view: None,
        //     },
        //     local_to_world: LocalToWorld::identity(),
        //     translation: Translation::new(4.0, -4.0, 5.0),
        //     rotation: Rotation::from_euler_angles(0.0, 0.0, 0.0),
        // })
        // camera
        .add_archetype(CameraEntity {
            camera: Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            active_camera: ActiveCamera,
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        })
    .build();
}