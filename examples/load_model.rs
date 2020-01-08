use bevy::*;

fn main() {
    // let universe = Universe::new();
    // let world = universe.create_world();
    // let scheduler = SystemScheduler::<ApplicationStage>::new();
    // asset::load_gltf(get_path("examples/assets/Box.gltf").to_str().unwrap()).unwrap();
    asset::load_gltf("examples/assets/Box.gltf").unwrap();

    // Application::run(universe, world, scheduler);
}