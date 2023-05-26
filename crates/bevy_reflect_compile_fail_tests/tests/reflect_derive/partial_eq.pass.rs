use bevy_reflect::prelude::*;

#[derive(Reflect, PartialEq, Clone)]
#[reflect_value(PartialEq)]
struct Foo {
    a: u32,
}

#[derive(Reflect)]
struct Bar {
    #[reflect(skip_partial_eq)]
    a: u32,
}

fn main() {}
