use bevy::*;
use bevy::{render::*, asset::{Asset, AssetStorage}, math};

// fn build_move_system() -> Box<dyn Scheduleable> {
//     SystemBuilder::new("MoveSystem")
//         .with_query(<>)
// }

fn main() {
    let universe = Universe::new();
    let mut world = universe.create_world();
    // Create a query which finds all `Position` and `Velocity` components
    // let mut query = Read::<Transform>::query();
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane{ size: 10 });
    let mut mesh_storage = AssetStorage::<Mesh, MeshType>::new();

    // this currently breaks because Arcs cant be modified after they are cloned :(
    let mesh_handle = mesh_storage.add(cube);
    let plane_handle = mesh_storage.add(plane);
    world.resources.insert(mesh_storage);

    world.insert((), vec![
        // plane
        (
            Material::new(math::vec4(0.1, 0.2, 0.1, 1.0)),
            plane_handle.clone(),
            LocalToWorld(math::translation(&math::vec3(0.0, 0.0, 0.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
        // cubes
        (
            Material::new(math::vec4(0.1, 0.1, 0.6, 1.0)),
            mesh_handle.clone(),
            LocalToWorld(math::translation(&math::vec3(1.5, 0.0, 1.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
        (
            Material::new(math::vec4(0.6, 0.1, 0.1, 1.0)),
            mesh_handle,
            LocalToWorld(math::translation(&math::vec3(-1.5, 0.0, 1.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
    ]);
    world.insert((), vec![
        // lights
        (
            Light {
                pos: math::vec3(7.0, -5.0, 10.0),
                color: wgpu::Color {
                    r: 0.5,
                    g: 1.0,
                    b: 0.5,
                    a: 1.0,
                },
                fov: f32::to_radians(60.0),
                depth: 1.0 .. 20.0,
                target_view: None,
            },
            LocalToWorld(math::translation(&math::vec3(7.0, -5.0, 10.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
        (
            Light {
                pos: math::vec3(-5.0, 7.0, 10.0),
                color: wgpu::Color {
                    r: 1.0,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                },
                fov: f32::to_radians(45.0),
                depth: 1.0 .. 20.0,
                target_view: None,
            },
            LocalToWorld(math::translation(&math::vec3(-1.5, 0.0, 1.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
    ]);
    world.insert((), vec![
        // camera
        (
            Camera::new(CameraType::Projection {
                fov: math::quarter_pi(),
                near: 1.0,
                far: 20.0,
                aspect_ratio: 1.0,
            }),
            LocalToWorld(math::look_at_rh(&math::vec3(3.0, -10.0, 6.0),
            &math::vec3(0.0, 0.0, 0.0),
            &math::vec3(0.0, 0.0, 1.0),)),
            Translation::new(0.0, 0.0, 0.0)
        )
    ]);

    // let transform_system_bundle = transform_system_bundle::build(&mut world);
    Application::run(universe, world);
}