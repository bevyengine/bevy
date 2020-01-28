use bevy::prelude::*;
use bevy::render::render_graph_2::{StandardMaterial, ShaderUniforms, uniform_selector};

fn main() {
    AppBuilder::new().add_defaults().add_system(build_move_system()).setup_world(setup).run();
}

fn build_move_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("Move")
        .read_resource::<Time>()
        .with_query(<(Write<Translation>, Write<StandardMaterial>)>::query())
        .build(move |_, world, time, person_query| {
            for (mut translation, mut material) in person_query.iter_mut(world) {
                translation.0 += math::vec3(1.0, 0.0, 0.0) * time.delta_seconds;
                material.albedo = material.albedo + math::vec4(-time.delta_seconds, -time.delta_seconds, time.delta_seconds, 0.0);
            }
        })
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane { size: 10.0 });

    let (cube_handle, plane_handle) = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        (mesh_storage.add(cube), mesh_storage.add(plane))
    };

    world.build()
        // plane
        .add_archetype(NewMeshEntity {
            mesh: plane_handle.clone(),
            material: StandardMaterial {
                albedo: math::vec4(0.1, 0.2, 0.1, 1.0),
            },
            shader_uniforms: ShaderUniforms {
                uniform_selectors: vec![
                   uniform_selector::<StandardMaterial>, 
                   uniform_selector::<LocalToWorld>, 
                ],
            },
            local_to_world: LocalToWorld::identity(),
            translation: Translation::new(0.0, 0.0, 0.0),
        })
        // cube
        .add_archetype(NewMeshEntity {
            mesh: cube_handle.clone(),
            material: StandardMaterial {
                albedo: math::vec4(1.0, 0.0, 0.0, 1.0),
            },
            shader_uniforms: ShaderUniforms {
                uniform_selectors: vec![
                   uniform_selector::<StandardMaterial>, 
                   uniform_selector::<LocalToWorld>, 
                ],
            },
            local_to_world: LocalToWorld::identity(),
            translation: Translation::new(0.0, 0.0, 1.0),
        })
        .add_archetype(NewMeshEntity {
            mesh: cube_handle.clone(),
            material: StandardMaterial {
                albedo: math::vec4(0.0, 1.0, 0.0, 1.0),
            },
            shader_uniforms: ShaderUniforms {
                uniform_selectors: vec![
                   uniform_selector::<StandardMaterial>, 
                   uniform_selector::<LocalToWorld>, 
                ],
            },
            local_to_world: LocalToWorld::identity(),
            translation: Translation::new(-2.0, 0.0, 1.0),
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
