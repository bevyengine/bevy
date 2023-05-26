use bevy_reflect::prelude::*;

// Reason: `#[reflect(Hash)]` only supported for value types
#[derive(Reflect, Hash)]
#[reflect(Hash)]
struct Foo {
    a: u32,
}

#[derive(Reflect)]
struct Bar {
    // Reason: `#[reflect(skip_hash)]` does not take any arguments
    #[reflect(skip_hash = true)]
    a: u32,
}

fn main() {}
