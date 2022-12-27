use bevy_reflect::Reflect;

#[derive(Reflect)]
struct Foo {
    #[reflect(ignore)]
    a: i32,
    b: i32,
}

fn main() {}
