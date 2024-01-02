use bevy_reflect::{FromReflect, Reflect};

// Reason: Cannot have conflicting `from_reflect` attributes
#[derive(Reflect)]
#[reflect(from_reflect = false)]
#[reflect(from_reflect = true)]
struct Foo {
    value: String,
}

// Reason: Cannot have conflicting `from_reflect` attributes
#[derive(Reflect)]
#[reflect(from_reflect = true)]
#[reflect(from_reflect = false)]
struct Bar {
    value: String,
}

// Reason: Conflicting `FromReflect` implementations
#[derive(Reflect, FromReflect)]
struct Baz {
    value: String,
}

fn main() {}
