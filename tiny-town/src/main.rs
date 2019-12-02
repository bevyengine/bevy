use bevy::*;
use bevy::{render::mesh::{Mesh, MeshType}, asset::{Asset, AssetStorage}, temp::*, math};

fn main() {
    let universe = Universe::new();
    let mut world = universe.create_world();
    // Create a query which finds all `Position` and `Velocity` components
    // let mut query = Read::<Transform>::query();
    let cube = Mesh::load(MeshType::Cube);
    let mut mesh_storage = AssetStorage::<Mesh, MeshType>::new();
    let handle = mesh_storage.add(cube);
    world.resources.insert(mesh_storage);

    world.insert((), vec![
        (CubeEnt { color: math::Vec4::identity(), bind_group: None, uniform_buf: None }, handle, LocalToWorld::identity(), Translation::new(0.0, 0.0, 0.0))
    ]);

    Application::run(universe, world);
}