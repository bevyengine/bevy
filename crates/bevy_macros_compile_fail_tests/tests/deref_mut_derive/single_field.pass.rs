use bevy_derive::DerefMut;
use std::ops::Deref;

#[derive(DerefMut)]
struct TupleStruct(#[deref] String);

impl Deref for TupleStruct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(DerefMut)]
struct Struct {
    #[deref]
    bar: String,
}

impl Deref for Struct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.bar
    }
}

fn main() {
    let mut value = TupleStruct("Hello world!".to_string());
    let _: &mut String = &mut *value;

    let mut value = Struct {
        bar: "Hello world!".to_string(),
    };
    let _: &mut String = &mut *value;
}
