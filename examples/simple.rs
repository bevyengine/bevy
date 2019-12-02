use bevy::*;
use bevy::{render::*, asset::{Asset, AssetStorage}, math};

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
        (
            Material { color: math::vec4(0.1, 0.2, 0.1, 1.0), bind_group: None, uniform_buf: None },
            plane_handle.clone(),
            LocalToWorld(math::translation(&math::vec3(0.0, 0.0, 0.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
        (
            Material { color: math::vec4(0.1, 0.1, 0.6, 1.0), bind_group: None, uniform_buf: None },
            mesh_handle.clone(),
            LocalToWorld(math::translation(&math::vec3(1.5, 0.0, 1.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
        (
            Material { color: math::vec4(0.6, 0.1, 0.1, 1.0), bind_group: None, uniform_buf: None },
            mesh_handle,
            LocalToWorld(math::translation(&math::vec3(-1.5, 0.0, 1.0))),
            Translation::new(0.0, 0.0, 0.0)
        ),
    ]);

    Application::run(universe, world);
}