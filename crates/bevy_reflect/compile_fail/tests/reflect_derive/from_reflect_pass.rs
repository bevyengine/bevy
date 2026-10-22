//@check-pass
use bevy_reflect::{FromDynamic, Reflect};

#[derive(Reflect)]
#[reflect(from_dynamic = false)]
#[reflect(from_dynamic = false)]
struct Foo {
    value: String,
}

#[derive(Reflect)]
#[reflect(from_dynamic = true)]
#[reflect(from_dynamic = true)]
struct Bar {
    value: String,
}

#[derive(Reflect, FromDynamic)]
#[reflect(from_dynamic = false)]
struct Baz {
    value: String,
}
