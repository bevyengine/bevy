use bevy_reflect::Reflect;

#[derive(Reflect)]
struct Foo {
    a: i32,
    #[reflect(ignore)]
    b: i32,
}

fn main() {}
