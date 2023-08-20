use bevy_derive::Deref;

#[derive(Deref)]
struct TupleStruct(usize, String);

#[derive(Deref)]
struct Struct {
    foo: usize,
    bar: String,
}

fn main() {}
