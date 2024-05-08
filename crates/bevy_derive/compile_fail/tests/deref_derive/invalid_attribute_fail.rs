use bevy_derive::Deref;

// Reason: `#[deref]` doesn't take any arguments

#[derive(Deref)]
struct TupleStruct(
    usize,
    #[deref()] String
    //~^ ERROR: unexpected token
);

#[derive(Deref)]
struct Struct {
    foo: usize,
    #[deref()]
    //~^ ERROR: unexpected token
    bar: String,
}
