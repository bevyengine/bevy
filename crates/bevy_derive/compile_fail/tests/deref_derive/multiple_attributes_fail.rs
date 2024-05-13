use bevy_derive::Deref;

#[derive(Deref)]
struct TupleStruct(
    #[deref] usize,
    #[deref] String
    //~^ ERROR: can only be used on a single field
);

#[derive(Deref)]
struct Struct {
    #[deref]
    foo: usize,
    #[deref]
    //~^ ERROR: can only be used on a single field
    bar: String,
}
