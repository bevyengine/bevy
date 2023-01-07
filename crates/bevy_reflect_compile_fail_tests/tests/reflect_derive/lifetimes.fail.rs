use bevy_reflect::Reflect;

#[derive(Reflect)]
struct Foo<'a> {
    #[reflect(ignore)]
    value: &'a str,
}

fn main() {}
