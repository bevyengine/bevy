//! Illustrates how to specify conversions for `FromReflect`

use bevy::{prelude::*, reflect::structs::DynamicStruct};

fn main() {
    App::new()
        .register_type::<Foo>()
        .add_systems(Startup, setup)
        .run();
}

/// Deriving `Reflect` implements the relevant reflection traits. In this case, it implements the
/// `Reflect` trait, the `Struct` trait, and the `FromReflect` trait. `derive(Reflect)` assumes that
/// all fields also implement `PartialReflect`.
///
/// You can specify additional types that will be accepted per field using `#[reflect(from(...))]`.
/// The additional types will be used when you call `YourType::from_reflect`.
#[derive(Reflect)]
pub struct Foo {
    // allow conversion using the From trait
    #[reflect(from(usize))]
    nested: Bar,
    // allow conversion using the provided function
    #[reflect(from(usize(|n| Bar{ a: n / 2 })))]
    nested2: Bar,
    // uses conversion specified in the type
    nested3: Baz,
}

/// This `Bar` type is used in the `nested` field of the `Foo` type. We must derive `Reflect` here
/// too (or ignore it)
#[derive(Reflect)]
pub struct Bar {
    a: usize,
}

impl From<usize> for Bar {
    fn from(value: usize) -> Bar {
        Bar { a: value }
    }
}

/// Reflection conversions can also be specified at the type-level.
#[derive(Reflect)]
#[reflect(from(usize))]
pub struct Baz {
    b: usize,
}

impl From<usize> for Baz {
    fn from(value: usize) -> Baz {
        Baz { b: value }
    }
}

fn setup() {
    // You can make a brand new instance using the `FromReflect` trait. It will use
    // the provided conversions if the types do not match exactly.
    let mut dynamic = DynamicStruct::default();
    dynamic.insert("nested", 42usize);
    dynamic.insert("nested2", 42usize);
    dynamic.insert("nested3", 42usize);

    let concrete = Foo::from_reflect(&dynamic).unwrap();
    assert_eq!(concrete.nested.a, 42);
    assert_eq!(concrete.nested2.a, 21);
    assert_eq!(concrete.nested3.b, 42);
}
