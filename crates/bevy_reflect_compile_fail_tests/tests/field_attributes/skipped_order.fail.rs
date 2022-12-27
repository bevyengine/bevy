use bevy_reflect::Reflect;

#[derive(Reflect)]
struct Foo {
    #[reflect(skip_serializing)]
    a: i32,
    b: i32,
}

#[derive(Reflect)]
struct Bar {
    a: i32,
    #[reflect(ignore)]
    b: i32,
    #[reflect(skip_serializing)]
    c: i32,
}

fn main() {}
