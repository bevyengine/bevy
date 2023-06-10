use bevy_derive::DerefMut;
use std::ops::Deref;

#[derive(DerefMut)]
struct TupleStruct(usize, #[deref] String);

impl Deref for TupleStruct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(DerefMut)]
struct Struct {
    foo: usize,
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
    let mut value = TupleStruct(123, "Hello world!".to_string());
    let _: &mut String = &mut *value;

    let mut value = Struct {
        foo: 123,
        bar: "Hello world!".to_string(),
    };
    let _: &mut String = &mut *value;
}
