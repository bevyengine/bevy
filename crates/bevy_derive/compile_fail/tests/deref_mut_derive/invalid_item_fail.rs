use bevy_derive::DerefMut;

#[derive(DerefMut)]
//~^ ERROR: cannot be derived on field-less structs
struct UnitStruct;

#[derive(DerefMut)]
//~^ ERROR: can only be derived on structs
enum Enum {}
