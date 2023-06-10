use bevy_derive::DerefMut;
use std::ops::Deref;

// Reason: `#[deref]` doesn't take any arguments

#[derive(DerefMut)]
struct TupleStruct(usize, #[deref()] String);

impl Deref for TupleStruct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(DerefMut)]
struct Struct {
    foo: usize,
    #[deref()]
    bar: String,
}

impl Deref for Struct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.bar
    }
}

fn main() {}
