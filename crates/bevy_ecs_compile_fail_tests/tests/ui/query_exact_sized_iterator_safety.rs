use bevy_ecs::prelude::*;

#[derive(Component)]
struct Foo;

fn on_changed(query: Query<&Foo, Changed<Foo>>) {
    // this should fail to compile
    println!("{}", query.iter().len())
}

fn on_added(query: Query<&Foo, Added<Foo>>) {
    // this should fail to compile
    println!("{}", query.iter().len())
}

fn main() {}