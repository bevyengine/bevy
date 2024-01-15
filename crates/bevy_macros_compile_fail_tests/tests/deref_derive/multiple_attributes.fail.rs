use bevy_derive::Deref;

#[derive(Deref)]
struct TupleStruct(#[deref] usize, #[deref] String);

#[derive(Deref)]
struct Struct {
    #[deref]
    foo: usize,
    #[deref]
    bar: String,
}

fn main() {}
