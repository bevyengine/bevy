use bevy_derive::DerefMut;
use core::ops::Deref;

#[derive(DerefMut)]
//~^ E0308
struct TupleStruct(#[deref] usize, String);

impl Deref for TupleStruct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(DerefMut)]
//~^ E0308
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
