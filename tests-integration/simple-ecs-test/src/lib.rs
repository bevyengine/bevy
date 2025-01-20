#![allow(dead_code)]
use bevy::prelude::*;

#[derive(Component)]
struct MyComponent {
    value: f32,
}

#[derive(Resource)]
struct MyResource {
    value: f32,
}

fn hello_world(query: Query<&MyComponent>, resource: Res<MyResource>) {
    let component = query.iter().next().unwrap();
    let comp_value = component.value; // rust-analyzer suggestions work
    let res_value_deref = resource.value; // rust-analyzer suggestions don't work but ctrl+click works once it's written, also type inlay hints work correctly
    let res_value_direct = resource.into_inner().value; // rust-analyzer suggestions work
    println!(
        "hello world! Value: {} {} {}",
        comp_value, res_value_deref, res_value_direct
    );
}

fn spawn_component(mut commands: Commands) {
    commands.spawn(MyComponent { value: 10.0 });
}

#[test]
fn simple_ecs_test() {
    App::new()
        .insert_resource(MyResource { value: 5.0 })
        .add_systems(Startup, spawn_component)
        .add_systems(Update, hello_world)
        .run();
}
