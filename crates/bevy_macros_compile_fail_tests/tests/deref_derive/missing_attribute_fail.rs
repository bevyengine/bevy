use bevy_derive::Deref;

#[derive(Deref)]
//~^ ERROR: requires one field to have
struct TupleStruct(usize, String);

#[derive(Deref)]
//~^ ERROR: requires one field to have
struct Struct {
    foo: usize,
    bar: String,
}
