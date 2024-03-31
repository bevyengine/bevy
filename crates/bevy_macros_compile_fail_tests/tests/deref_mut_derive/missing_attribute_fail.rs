use bevy_derive::DerefMut;
use std::ops::Deref;

#[derive(DerefMut)]
//~^ ERROR: requires one field to have
struct TupleStruct(usize, String);

impl Deref for TupleStruct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(DerefMut)]
//~^ ERROR: requires one field to have
struct Struct {
    foo: usize,
    bar: String,
}

impl Deref for Struct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.bar
    }
}
