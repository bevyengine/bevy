use bevy_reflect::Reflect;

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

fn main() {}
