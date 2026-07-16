//@edition: 2024
use bevy_ecs::prelude::*;

trait Scene {}

#[derive(Resource)]
struct MyScene;
impl Scene for MyScene {}

fn make_scene(_res: Res<MyScene>) -> impl Scene {
    MyScene
}

fn main() {
    let mut schedule = Schedule::default();
    schedule.add_systems(make_scene);
    //~^ E0277
}
