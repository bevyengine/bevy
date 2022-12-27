use bevy_reflect::Reflect;

#[derive(Reflect)]
struct Foo {
    a: i32,
    #[reflect(skip_serializing)]
    b: i32,
}

#[derive(Reflect)]
struct Bar {
    a: i32,
    #[reflect(skip_serializing)]
    b: i32,
    #[reflect(ignore)]
    c: i32,
}

fn main() {}
