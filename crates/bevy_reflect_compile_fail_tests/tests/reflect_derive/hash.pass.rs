use bevy_reflect::prelude::*;

#[derive(Reflect, Hash, Clone)]
#[reflect_value(Hash)]
struct Foo {
    a: u32,
}

#[derive(Reflect)]
struct Bar {
    #[reflect(skip_hash)]
    a: u32,
}

fn main() {}
