use bevy_derive::Deref;

// Reason: `#[deref]` doesn't take any arguments

#[derive(Deref)]
struct TupleStruct(usize, #[deref()] String);

#[derive(Deref)]
struct Struct {
    foo: usize,
    #[deref()]
    bar: String,
}

fn main() {}
