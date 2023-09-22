use bevy_derive::DerefMut;
use std::ops::Deref;

#[derive(DerefMut)]
struct TupleStruct(#[deref] usize, String);

impl Deref for TupleStruct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(DerefMut)]
struct Struct {
    #[deref]
    foo: usize,
    bar: String,
}

impl Deref for Struct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.bar
    }
}

fn main() {}
