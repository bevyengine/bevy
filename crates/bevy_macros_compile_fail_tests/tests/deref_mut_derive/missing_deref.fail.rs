use bevy_derive::DerefMut;

#[derive(DerefMut)]
struct TupleStruct(usize, #[deref] String);

#[derive(DerefMut)]
struct Struct {
    foo: usize,
    #[deref]
    bar: String,
}

fn main() {}
