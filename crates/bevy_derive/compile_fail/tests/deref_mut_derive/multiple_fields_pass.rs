//@check-pass

use bevy_derive::DerefMut;
use std::ops::Deref;

#[derive(DerefMut)]
// The first field is never read, but we want it there to check that the derive skips it.
struct TupleStruct(#[allow(dead_code)] usize, #[deref] String);

impl Deref for TupleStruct {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

#[derive(DerefMut)]
struct Struct {
    #[allow(dead_code)]
    // Same justification as above.
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
