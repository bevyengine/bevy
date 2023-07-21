use bevy_reflect::prelude::*;

// Reason: `#[reflect(PartialEq)]` only supported for value types
#[derive(Reflect, PartialEq)]
#[reflect(PartialEq)]
struct Foo {
    a: u32,
}

#[derive(Reflect)]
struct Bar {
    // Reason: `#[reflect(skip_partial_eq)]` does not take any arguments
    #[reflect(skip_partial_eq = true)]
    a: u32,
}

fn main() {}
