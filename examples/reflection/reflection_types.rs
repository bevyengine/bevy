#![allow(clippy::match_same_arms)]
//! This example illustrates how reflection works for simple data structures, like
//! structs, tuples and vectors.

use bevy::{
    prelude::*,
    reflect::{DynamicList, ReflectRef},
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// Deriving reflect on a struct will implement the `Reflect` and `Struct` traits
#[derive(Reflect)]
pub struct A {
    x: usize,
    y: Vec<u32>,
    z: HashMap<String, f32>,
}

/// Deriving reflect on a unit struct will implement the `Reflect` and `Struct` traits
#[derive(Reflect)]
pub struct B;

/// Deriving reflect on a tuple struct will implement the `Reflect` and `TupleStruct` traits
#[derive(Reflect)]
pub struct C(usize);

/// Deriving reflect on an enum will implement the `Reflect` and `Enum` traits
#[derive(Reflect)]
enum D {
    A,
    B(usize),
    C { value: f32 },
}

/// Reflect has "built in" support for some common traits like `PartialEq`, `Hash`, and `Serialize`.
/// These are exposed via methods like `Reflect::reflect_hash()`, `Reflect::reflect_partial_eq()`, and
/// `Reflect::serializable()`. You can force these implementations to use the actual trait
/// implementations (instead of their defaults) like this:
#[derive(Reflect, Hash, Serialize, PartialEq, Eq)]
#[reflect(Hash, Serialize, PartialEq)]
pub struct E {
    x: usize,
}

/// By default, deriving with Reflect assumes the type is either a "struct" or an "enum".
/// You can tell reflect to treat your type instead as a "value type" by using the `reflect_value`
/// attribute in place of `reflect`. It is generally a good idea to implement (and reflect)
/// the `PartialEq`, `Serialize`, and `Deserialize` traits on `reflect_value` types to ensure
/// that these values behave as expected when nested underneath Reflect-ed structs.
#[derive(Reflect, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[reflect_value(PartialEq, Serialize, Deserialize)]
enum F {
    X,
    Y,
}

fn setup() {
    let mut z = HashMap::default();
    z.insert("Hello".to_string(), 1.0);
    let value: Box<dyn Reflect> = Box::new(A {
        x: 1,
        y: vec![1, 2],
        z,
    });

    // There are a number of different "reflect traits", which each expose different operations on
    // the underlying type
    match value.reflect_ref() {
        // `Struct` is a trait automatically implemented for structs that derive Reflect. This trait
        // allows you to interact with fields via their string names or indices
        ReflectRef::Struct(value) => {
            info!(
                "This is a 'struct' type with an 'x' value of {}",
                value.get_field::<usize>("x").unwrap()
            );
        }
        // `TupleStruct` is a trait automatically implemented for tuple structs that derive Reflect.
        // This trait allows you to interact with fields via their indices
        ReflectRef::TupleStruct(_) => {}
        // `Tuple` is a special trait that can be manually implemented (instead of deriving
        // Reflect). This exposes "tuple" operations on your type, allowing you to interact
        // with fields via their indices. Tuple is automatically implemented for tuples of
        // arity 12 or less.
        ReflectRef::Tuple(_) => {}
        // `Enum` is a trait automatically implemented for enums that derive Reflect. This trait allows you
        // to interact with the current variant and its fields (if it has any)
        ReflectRef::Enum(_) => {}
        // `List` is a special trait that can be manually implemented (instead of deriving Reflect).
        // This exposes "list" operations on your type, such as insertion. `List` is automatically
        // implemented for relevant core types like Vec<T>.
        ReflectRef::List(_) => {}
        // `Array` is a special trait that can be manually implemented (instead of deriving Reflect).
        // This exposes "array" operations on your type, such as indexing. `Array`
        // is automatically implemented for relevant core types like [T; N].
        ReflectRef::Array(_) => {}
        // `Map` is a special trait that can be manually implemented (instead of deriving Reflect).
        // This exposes "map" operations on your type, such as getting / inserting by key.
        // Map is automatically implemented for relevant core types like HashMap<K, V>
        ReflectRef::Map(_) => {}
        // `Value` types do not implement any of the other traits above. They are simply a Reflect
        // implementation. Value is implemented for core types like i32, usize, f32, and
        // String.
        ReflectRef::Value(_) => {}
    }

    let mut dynamic_list = DynamicList::default();
    dynamic_list.push(3u32);
    dynamic_list.push(4u32);
    dynamic_list.push(5u32);

    let mut value: A = value.take::<A>().unwrap();
    value.y.apply(&dynamic_list);
    assert_eq!(value.y, vec![3u32, 4u32, 5u32]);
}
