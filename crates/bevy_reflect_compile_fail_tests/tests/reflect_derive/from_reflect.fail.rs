use bevy_reflect::Reflect;

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

fn main() {}
