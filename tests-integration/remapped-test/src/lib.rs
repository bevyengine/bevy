#![allow(dead_code)]
use bevi::prelude::*;

#[derive(Component)]
struct _Component {
    _value: f32,
}

#[derive(Resource)]
struct _Resource {
    _value: f32,
}

fn hello_world() {
    println!("hello world!");
}

#[test]
fn simple_ecs_test() {
    App::new().add_systems(Update, hello_world).run();
}
