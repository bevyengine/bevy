use bevy_derive::Deref;

#[derive(Deref)]
//~^ ERROR: cannot be derived on field-less structs
struct UnitStruct;

#[derive(Deref)]
//~^ ERROR: can only be derived on structs
enum Enum {}
