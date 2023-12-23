use bevy_reflect::{FromReflect, Reflect};

#[derive(Reflect)]
#[reflect(from_reflect = false)]
#[reflect(from_reflect = false)]
struct Foo {
    value: String,
}

#[derive(Reflect)]
#[reflect(from_reflect = true)]
#[reflect(from_reflect = true)]
struct Bar {
    value: String,
}

#[derive(Reflect, FromReflect)]
#[reflect(from_reflect = false)]
struct Baz {
    value: String,
}

fn main() {}
