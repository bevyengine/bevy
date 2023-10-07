//! Reflection in Rust.
//!
//! [Reflection] is a powerful tool provided within many programming languages
//! that allows for meta-programming: using information _about_ the program to
//! _affect_ the program.
//! In other words, reflection allows us to inspect the program itself, its
//! syntax, and its type information at runtime.
//!
//! This crate adds this missing reflection functionality to Rust.
//! Though it was made with the [Bevy] game engine in mind,
//! it's a general-purpose solution that can be used in any Rust project.
//!
//! At a very high level, this crate allows you to:
//! * Dynamically interact with Rust values
//! * Access type metadata at runtime
//! * Serialize and deserialize (i.e. save and load) data
//!
//! It's important to note that because of missing features in Rust,
//! there are some [limitations] with this crate.
//!
//! # The `Reflect` Trait
//!
//! At the core of [`bevy_reflect`] is the [`Reflect`] trait.
//!
//! One of its primary purposes is to allow all implementors to be passed around
//! as a `dyn Reflect` trait object.
//! This allows any such type to be operated upon completely dynamically (at a small [runtime cost]).
//!
//! Implementing the trait is easily done using the provided [derive macro]:
//!
//! ```
//! # use bevy_reflect::Reflect;
//! #[derive(Reflect)]
//! struct MyStruct {
//!   foo: i32
//! }
//! ```
//!
//! This will automatically generate the implementation of `Reflect` for any struct or enum.
//!
//! It will also generate other very important trait implementations used for reflection:
//! * [`GetTypeRegistration`]
//! * [`Typed`]
//! * [`Struct`], [`TupleStruct`], or [`Enum`] depending on the type
//!
//! ## Requirements
//!
//! We can implement `Reflect` on any type that satisfies _both_ of the following conditions:
//! * The type implements `Any`.
//!   This is true if and only if the type itself has a [`'static` lifetime].
//! * All fields and sub-elements themselves implement `Reflect`
//!   (see the [derive macro documentation] for details on how to ignore certain fields when deriving).
//!
//! Additionally, using the derive macro on enums requires a third condition to be met:
//! * All fields and sub-elements must implement [`FromReflect`]—
//! another important reflection trait discussed in a later section.
//!
//! # The `Reflect` Subtraits
//!
//! Since [`Reflect`] is meant to cover any and every type, this crate also comes with a few
//! more traits to accompany `Reflect` and provide more specific interactions.
//! We refer to these traits as the _reflection subtraits_ since they all have `Reflect` as a supertrait.
//! The current list of reflection subtraits include:
//! * [`Tuple`]
//! * [`Array`]
//! * [`List`]
//! * [`Map`]
//! * [`Struct`]
//! * [`TupleStruct`]
//! * [`Enum`]
//!
//! As mentioned previously, the last three are automatically implemented by the [derive macro].
//!
//! Each of these traits come with their own methods specific to their respective category.
//! For example, we can access our struct's fields by name using the [`Struct::field`] method.
//!
//! ```
//! # use bevy_reflect::{Reflect, Struct};
//! # #[derive(Reflect)]
//! # struct MyStruct {
//! #   foo: i32
//! # }
//! let my_struct: Box<dyn Struct> = Box::new(MyStruct {
//!   foo: 123
//! });
//! let foo: &dyn Reflect = my_struct.field("foo").unwrap();
//! assert_eq!(Some(&123), foo.downcast_ref::<i32>());
//! ```
//!
//! Since most data is passed around as `dyn Reflect`,
//! the `Reflect` trait has methods for going to and from these subtraits.
//!
//! [`Reflect::reflect_ref`], [`Reflect::reflect_mut`], and [`Reflect::reflect_owned`] all return
//! an enum that respectively contains immutable, mutable, and owned access to the type as a subtrait object.
//!
//! For example, we can get out a `dyn Tuple` from our reflected tuple type using one of these methods.
//!
//! ```
//! # use bevy_reflect::{Reflect, ReflectRef};
//! let my_tuple: Box<dyn Reflect> = Box::new((1, 2, 3));
//! let ReflectRef::Tuple(my_tuple) = my_tuple.reflect_ref() else { unreachable!() };
//! assert_eq!(3, my_tuple.field_len());
//! ```
//!
//! And to go back to a general-purpose `dyn Reflect`,
//! we can just use the matching [`Reflect::as_reflect`], [`Reflect::as_reflect_mut`],
//! or [`Reflect::into_reflect`] methods.
//!
//! ## Value Types
//!
//! Types that do not fall under one of the above subtraits,
//! such as for primitives (e.g. `bool`, `usize`, etc.)
//! and simple types (e.g. `String`, `Duration`),
//! are referred to as _value_ types
//! since methods like [`Reflect::reflect_ref`] return a [`ReflectRef::Value`] variant.
//! While most other types contain their own `dyn Reflect` fields and data,
//! these types generally cannot be broken down any further.
//!
//! # Dynamic Types
//!
//! Each subtrait comes with a corresponding _dynamic_ type.
//!
//! The available dynamic types are:
//! * [`DynamicTuple`]
//! * [`DynamicArray`]
//! * [`DynamicList`]
//! * [`DynamicMap`]
//! * [`DynamicStruct`]
//! * [`DynamicTupleStruct`]
//! * [`DynamicEnum`]
//!
//! These dynamic types may contain any arbitrary reflected data.
//!
//! ```
//! # use bevy_reflect::{DynamicStruct, Struct};
//! let mut data = DynamicStruct::default();
//! data.insert("foo", 123_i32);
//! assert_eq!(Some(&123), data.field("foo").unwrap().downcast_ref::<i32>())
//! ```
//!
//! They are most commonly used as "proxies" for other types,
//! where they contain the same data as— and therefore, represent— a concrete type.
//! The [`Reflect::clone_value`] method will return a dynamic type for all non-value types,
//! allowing all types to essentially be "cloned".
//! And since dynamic types themselves implement [`Reflect`],
//! we may pass them around just like any other reflected type.
//!
//! ```
//! # use bevy_reflect::{DynamicStruct, Reflect};
//! # #[derive(Reflect)]
//! # struct MyStruct {
//! #   foo: i32
//! # }
//! let original: Box<dyn Reflect> = Box::new(MyStruct {
//!   foo: 123
//! });
//!
//! // `cloned` will be a `DynamicStruct` representing a `MyStruct`
//! let cloned: Box<dyn Reflect> = original.clone_value();
//! assert!(cloned.represents::<MyStruct>());
//! assert!(cloned.is::<DynamicStruct>());
//! ```
//!
//! ## Patching
//!
//! These dynamic types come in handy when needing to apply multiple changes to another type.
//! This is known as "patching" and is done using the [`Reflect::apply`] method.
//!
//! ```
//! # use bevy_reflect::{DynamicEnum, Reflect};
//! let mut value = Some(123_i32);
//! let patch = DynamicEnum::new("None", ());
//! value.apply(&patch);
//! assert_eq!(None, value);
//! ```
//!
//! ## `FromReflect`
//!
//! It's important to remember that dynamic types are _not_ the concrete type they may be representing.
//! A common mistake is to treat them like such when trying to cast back to the original type
//! or when trying to make use of a reflected trait which expects the actual type.
//!
//! ```should_panic
//! # use bevy_reflect::{DynamicStruct, Reflect};
//! # #[derive(Reflect)]
//! # struct MyStruct {
//! #   foo: i32
//! # }
//! let original: Box<dyn Reflect> = Box::new(MyStruct {
//!   foo: 123
//! });
//!
//! let cloned: Box<dyn Reflect> = original.clone_value();
//! let value = cloned.take::<MyStruct>().unwrap(); // PANIC!
//! ```
//!
//! To resolve this issue, we'll need to convert the dynamic type to the concrete one.
//! This is where [`FromReflect`] comes in.
//!
//! `FromReflect` is a trait that allows an instance of a type to be generated from a
//! dynamic representation— even partial ones.
//! And since the [`FromReflect::from_reflect`] method takes the data by reference,
//! this can be used to effectively clone data (to an extent).
//!
//! It is automatically implemented when [deriving `Reflect`] on a type unless opted out of
//! using `#[reflect(from_reflect = false)]` on the item.
//!
//! ```
//! # use bevy_reflect::{Reflect, FromReflect};
//! #[derive(Reflect)]
//! struct MyStruct {
//!   foo: i32
//! }
//! let original: Box<dyn Reflect> = Box::new(MyStruct {
//!   foo: 123
//! });
//!
//! let cloned: Box<dyn Reflect> = original.clone_value();
//! let value = <MyStruct as FromReflect>::from_reflect(&*cloned).unwrap(); // OK!
//! ```
//!
//! When deriving, all active fields and sub-elements must also implement `FromReflect`.
//!
//! Fields can be given default values for when a field is missing in the passed value or even ignored.
//! Ignored fields must either implement [`Default`] or have a default function specified
//! using `#[reflect(default = "path::to::function")]`.
//!
//! See the [derive macro documentation](derive@crate::FromReflect) for details.
//!
//! All primitives and simple types implement `FromReflect` by relying on their [`Default`] implementation.
//!
//! # Path navigation
//!
//! The [`GetPath`] trait allows accessing arbitrary nested fields of a [`Reflect`] type.
//!
//! Using `GetPath`, it is possible to use a path string to access a specific field
//! of a reflected type.
//!
//! ```
//! # use bevy_reflect::{Reflect, GetPath};
//! #[derive(Reflect)]
//! struct MyStruct {
//!   value: Vec<Option<u32>>
//! }
//!
//! let my_struct = MyStruct {
//!   value: vec![None, None, Some(123)],
//! };
//! assert_eq!(
//!   my_struct.path::<u32>(".value[2].0").unwrap(),
//!   &123,
//! );
//! ```
//!
//! # Type Registration
//!
//! This crate also comes with a [`TypeRegistry`] that can be used to store and retrieve additional type metadata at runtime,
//! such as helper types and trait implementations.
//!
//! The [derive macro] for [`Reflect`] also generates an implementation of the [`GetTypeRegistration`] trait,
//! which is used by the registry to generate a [`TypeRegistration`] struct for that type.
//! We can then register additional [type data] we want associated with that type.
//!
//! For example, we can register [`ReflectDefault`] on our type so that its `Default` implementation
//! may be used dynamically.
//!
//! ```
//! # use bevy_reflect::{Reflect, TypeRegistry, prelude::ReflectDefault};
//! #[derive(Reflect, Default)]
//! struct MyStruct {
//!   foo: i32
//! }
//! let mut registry = TypeRegistry::empty();
//! registry.register::<MyStruct>();
//! registry.register_type_data::<MyStruct, ReflectDefault>();
//!
//! let registration = registry.get(std::any::TypeId::of::<MyStruct>()).unwrap();
//! let reflect_default = registration.data::<ReflectDefault>().unwrap();
//!
//! let new_value: Box<dyn Reflect> = reflect_default.default();
//! assert!(new_value.is::<MyStruct>());
//! ```
//!
//! Because this operation is so common, the derive macro actually has a shorthand for it.
//! By using the `#[reflect(Trait)]` attribute, the derive macro will automatically register a matching,
//! in-scope `ReflectTrait` type within the `GetTypeRegistration` implementation.
//!
//! ```
//! use bevy_reflect::prelude::{Reflect, ReflectDefault};
//!
//! #[derive(Reflect, Default)]
//! #[reflect(Default)]
//! struct MyStruct {
//!   foo: i32
//! }
//! ```
//!
//! ## Reflecting Traits
//!
//! Type data doesn't have to be tied to a trait, but it's often extremely useful to create trait type data.
//! These allow traits to be used directly on a `dyn Reflect` while utilizing the underlying type's implementation.
//!
//! For any [object-safe] trait, we can easily generate a corresponding `ReflectTrait` type for our trait
//! using the [`#[reflect_trait]`](reflect_trait) macro.
//!
//! ```
//! # use bevy_reflect::{Reflect, reflect_trait, TypeRegistry};
//! #[reflect_trait] // Generates a `ReflectMyTrait` type
//! pub trait MyTrait {}
//! impl<T: Reflect> MyTrait for T {}
//!
//! let mut registry = TypeRegistry::new();
//! registry.register_type_data::<i32, ReflectMyTrait>();
//! ```
//!
//! The generated type data can be used to convert a valid `dyn Reflect` into a `dyn MyTrait`.
//! See the [trait reflection example](https://github.com/bevyengine/bevy/blob/latest/examples/reflection/trait_reflection.rs)
//! for more information and usage details.
//!
//! # Serialization
//!
//! By using reflection, we are also able to get serialization capabilities for free.
//! In fact, using [`bevy_reflect`] can result in faster compile times and reduced code generation over
//! directly deriving the [`serde`] traits.
//!
//! The way it works is by moving the serialization logic into common serializers and deserializers:
//! * [`ReflectSerializer`]
//! * [`TypedReflectSerializer`]
//! * [`UntypedReflectDeserializer`]
//! * [`TypedReflectDeserializer`]
//!
//! All of these structs require a reference to the [registry] so that [type information] can be retrieved,
//! as well as registered type data, such as [`ReflectSerialize`] and [`ReflectDeserialize`].
//!
//! The general entry point are the "untyped" versions of these structs.
//! These will automatically extract the type information and pass them into their respective "typed" version.
//!
//! The output of the `ReflectSerializer` will be a map, where the key is the [type name]
//! and the value is the serialized data.
//! The `TypedReflectSerializer` will simply output the serialized data.
//!
//! The `UntypedReflectDeserializer` can be used to deserialize this map and return a `Box<dyn Reflect>`,
//! where the underlying type will be a dynamic type representing some concrete type (except for value types).
//!
//! Again, it's important to remember that dynamic types may need to be converted to their concrete counterparts
//! in order to be used in certain cases.
//! This can be achieved using [`FromReflect`].
//!
//! ```
//! # use serde::de::DeserializeSeed;
//! # use bevy_reflect::{
//! #     serde::{ReflectSerializer, UntypedReflectDeserializer},
//! #     Reflect, FromReflect, TypeRegistry
//! # };
//! #[derive(Reflect, PartialEq, Debug)]
//! struct MyStruct {
//!   foo: i32
//! }
//!
//! let original_value = MyStruct {
//!   foo: 123
//! };
//!
//! // Register
//! let mut registry = TypeRegistry::new();
//! registry.register::<MyStruct>();
//!
//! // Serialize
//! let reflect_serializer = ReflectSerializer::new(&original_value, &registry);
//! let serialized_value: String = ron::to_string(&reflect_serializer).unwrap();
//!
//! // Deserialize
//! let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
//! let deserialized_value: Box<dyn Reflect> = reflect_deserializer.deserialize(
//!   &mut ron::Deserializer::from_str(&serialized_value).unwrap()
//! ).unwrap();
//!
//! // Convert
//! let converted_value = <MyStruct as FromReflect>::from_reflect(&*deserialized_value).unwrap();
//!
//! assert_eq!(original_value, converted_value);
//! ```
//!
//! # Limitations
//!
//! While this crate offers a lot in terms of adding reflection to Rust,
//! it does come with some limitations that don't make it as featureful as reflection
//! in other programming languages.
//!
//! ## Non-Static Lifetimes
//!
//! One of the most obvious limitations is the `'static` requirement.
//! Rust requires fields to define a lifetime for referenced data,
//! but [`Reflect`] requires all types to have a `'static` lifetime.
//! This makes it impossible to reflect any type with non-static borrowed data.
//!
//! ## Function Reflection
//!
//! Another limitation is the inability to fully reflect functions and methods.
//! Most languages offer some way of calling methods dynamically,
//! but Rust makes this very difficult to do.
//! For non-generic methods, this can be done by registering custom [type data] that
//! contains function pointers.
//! For generic methods, the same can be done but will typically require manual monomorphization
//! (i.e. manually specifying the types the generic method can take).
//!
//! ## Manual Registration
//!
//! Since Rust doesn't provide built-in support for running initialization code before `main`,
//! there is no way for `bevy_reflect` to automatically register types into the [type registry].
//! This means types must manually be registered, including their desired monomorphized
//! representations if generic.
//!
//! # Features
//!
//! ## `bevy`
//!
//! | Default | Dependencies                              |
//! | :-----: | :---------------------------------------: |
//! | ❌      | [`bevy_math`], [`glam`], [`smallvec`] |
//!
//! This feature makes it so that the appropriate reflection traits are implemented on all the types
//! necessary for the [Bevy] game engine.
//! enables the optional dependencies: [`bevy_math`], [`glam`], and [`smallvec`].
//! These dependencies are used by the [Bevy] game engine and must define their reflection implementations
//! within this crate due to Rust's [orphan rule].
//!
//! ## `documentation`
//!
//! | Default | Dependencies                                  |
//! | :-----: | :-------------------------------------------: |
//! | ❌      | [`bevy_reflect_derive/documentation`]         |
//!
//! This feature enables capturing doc comments as strings for items that [derive `Reflect`].
//! Documentation information can then be accessed at runtime on the [`TypeInfo`] of that item.
//!
//! This can be useful for generating documentation for scripting language interop or
//! for displaying tooltips in an editor.
//!
//! [Reflection]: https://en.wikipedia.org/wiki/Reflective_programming
//! [Bevy]: https://bevyengine.org/
//! [limitations]: #limitations
//! [`bevy_reflect`]: crate
//! [runtime cost]: https://doc.rust-lang.org/book/ch17-02-trait-objects.html#trait-objects-perform-dynamic-dispatch
//! [derive macro]: derive@crate::Reflect
//! [`'static` lifetime]: https://doc.rust-lang.org/rust-by-example/scope/lifetime/static_lifetime.html#trait-bound
//! [derive macro documentation]: derive@crate::Reflect
//! [deriving `Reflect`]: derive@crate::Reflect
//! [type data]: TypeData
//! [`ReflectDefault`]: std_traits::ReflectDefault
//! [object-safe]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
//! [`serde`]: ::serde
//! [`ReflectSerializer`]: serde::ReflectSerializer
//! [`TypedReflectSerializer`]: serde::TypedReflectSerializer
//! [`UntypedReflectDeserializer`]: serde::UntypedReflectDeserializer
//! [`TypedReflectDeserializer`]: serde::TypedReflectDeserializer
//! [registry]: TypeRegistry
//! [type information]: TypeInfo
//! [type name]: Reflect::type_name
//! [type registry]: TypeRegistry
//! [`bevy_math`]: https://docs.rs/bevy_math/latest/bevy_math/
//! [`glam`]: https://docs.rs/glam/latest/glam/
//! [`smallvec`]: https://docs.rs/smallvec/latest/smallvec/
//! [orphan rule]: https://doc.rust-lang.org/book/ch10-02-traits.html#implementing-a-trait-on-a-type:~:text=But%20we%20can%E2%80%99t,implementation%20to%20use.
//! [`bevy_reflect_derive/documentation`]: bevy_reflect_derive
//! [derive `Reflect`]: derive@crate::Reflect
#![allow(clippy::type_complexity)]

mod array;
mod fields;
mod from_reflect;
mod list;
mod map;
mod path;
mod reflect;
mod struct_trait;
mod tuple;
mod tuple_struct;
mod type_info;
mod type_path;
mod type_registry;
mod type_uuid;
mod type_uuid_impl;
mod impls {
    #[cfg(feature = "glam")]
    mod glam;
    #[cfg(feature = "bevy_math")]
    mod rect;
    #[cfg(feature = "smallvec")]
    mod smallvec;
    #[cfg(feature = "smol_str")]
    mod smol_str;

    mod std;
    mod uuid;

    #[cfg(feature = "glam")]
    pub use self::glam::*;
    #[cfg(feature = "bevy_math")]
    pub use self::rect::*;
    #[cfg(feature = "smallvec")]
    pub use self::smallvec::*;
    pub use self::std::*;
    pub use self::uuid::*;
}

mod enums;
pub mod serde;
pub mod std_traits;
pub mod utility;

pub mod prelude {
    pub use crate::std_traits::*;
    #[doc(hidden)]
    pub use crate::{
        reflect_trait, FromReflect, GetField, GetPath, GetTupleStructField, Reflect,
        ReflectDeserialize, ReflectFromReflect, ReflectPath, ReflectSerialize, Struct, TupleStruct,
        TypePath,
    };
}

pub use array::*;
pub use enums::*;
pub use fields::*;
pub use from_reflect::*;
pub use impls::*;
pub use list::*;
pub use map::*;
pub use path::*;
pub use reflect::*;
pub use struct_trait::*;
pub use tuple::*;
pub use tuple_struct::*;
pub use type_info::*;
pub use type_path::*;
pub use type_registry::*;
pub use type_uuid::*;

pub use bevy_reflect_derive::*;
pub use erased_serde;

extern crate alloc;

#[doc(hidden)]
pub mod __macro_exports {
    use crate::Uuid;

    /// Generates a new UUID from the given UUIDs `a` and `b`,
    /// where the bytes are generated by a bitwise `a ^ b.rotate_right(1)`.
    /// The generated UUID will be a `UUIDv4` (meaning that the bytes should be random, not e.g. derived from the system time).
    #[allow(clippy::unusual_byte_groupings)] // unusual byte grouping is meant to signal the relevant bits
    pub const fn generate_composite_uuid(a: Uuid, b: Uuid) -> Uuid {
        let mut new = [0; 16];
        let mut i = 0;
        while i < new.len() {
            // rotating ensures different uuids for A<B<C>> and B<A<C>> because: A ^ (B ^ C) = B ^ (A ^ C)
            // notice that you have to rotate the second parameter: A.rr ^ (B.rr ^ C) = B.rr ^ (A.rr ^ C)
            // Solution: A ^ (B ^ C.rr).rr != B ^ (A ^ C.rr).rr
            new[i] = a.as_bytes()[i] ^ b.as_bytes()[i].rotate_right(1);

            i += 1;
        }

        // Version: the most significant 4 bits in the 6th byte: 11110000
        new[6] = new[6] & 0b0000_1111 | 0b0100_0000; // set version to v4

        // Variant: the most significant 3 bits in the 8th byte: 11100000
        new[8] = new[8] & 0b000_11111 | 0b100_00000; // set variant to rfc4122

        Uuid::from_bytes(new)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_types, clippy::approx_constant)]
mod tests {
    #[cfg(feature = "glam")]
    use ::glam::{vec3, Vec3};
    use ::serde::{de::DeserializeSeed, Deserialize, Serialize};
    use bevy_utils::HashMap;
    use ron::{
        ser::{to_string_pretty, PrettyConfig},
        Deserializer,
    };
    use std::{
        any::TypeId,
        borrow::Cow,
        fmt::{Debug, Formatter},
        marker::PhantomData,
    };

    use super::prelude::*;
    use super::*;
    use crate as bevy_reflect;
    use crate::serde::{ReflectSerializer, UntypedReflectDeserializer};
    use crate::utility::GenericTypePathCell;

    #[test]
    fn reflect_struct() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            b: f32,
            c: Bar,
        }
        #[derive(Reflect)]
        struct Bar {
            x: u32,
        }

        let mut foo = Foo {
            a: 42,
            b: 3.14,
            c: Bar { x: 1 },
        };

        let a = *foo.get_field::<u32>("a").unwrap();
        assert_eq!(a, 42);

        *foo.get_field_mut::<u32>("a").unwrap() += 1;
        assert_eq!(foo.a, 43);

        let bar = foo.get_field::<Bar>("c").unwrap();
        assert_eq!(bar.x, 1);

        // nested retrieval
        let c = foo.field("c").unwrap();
        if let ReflectRef::Struct(value) = c.reflect_ref() {
            assert_eq!(*value.get_field::<u32>("x").unwrap(), 1);
        } else {
            panic!("Expected a struct.");
        }

        // patch Foo with a dynamic struct
        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.insert("a", 123u32);
        dynamic_struct.insert("should_be_ignored", 456);

        foo.apply(&dynamic_struct);
        assert_eq!(foo.a, 123);
    }

    #[test]
    fn reflect_map() {
        #[derive(Reflect, Hash)]
        #[reflect(Hash)]
        struct Foo {
            a: u32,
            b: String,
        }

        let key_a = Foo {
            a: 1,
            b: "k1".to_string(),
        };

        let key_b = Foo {
            a: 1,
            b: "k1".to_string(),
        };

        let key_c = Foo {
            a: 3,
            b: "k3".to_string(),
        };

        let mut map = DynamicMap::default();
        map.insert(key_a, 10u32);
        assert_eq!(10, *map.get(&key_b).unwrap().downcast_ref::<u32>().unwrap());
        assert!(map.get(&key_c).is_none());
        *map.get_mut(&key_b).unwrap().downcast_mut::<u32>().unwrap() = 20;
        assert_eq!(20, *map.get(&key_b).unwrap().downcast_ref::<u32>().unwrap());
    }

    #[test]
    #[allow(clippy::disallowed_types)]
    fn reflect_unit_struct() {
        #[derive(Reflect)]
        struct Foo(u32, u64);

        let mut foo = Foo(1, 2);
        assert_eq!(1, *foo.get_field::<u32>(0).unwrap());
        assert_eq!(2, *foo.get_field::<u64>(1).unwrap());

        let mut patch = DynamicTupleStruct::default();
        patch.insert(3u32);
        patch.insert(4u64);
        assert_eq!(3, *patch.field(0).unwrap().downcast_ref::<u32>().unwrap());
        assert_eq!(4, *patch.field(1).unwrap().downcast_ref::<u64>().unwrap());

        foo.apply(&patch);
        assert_eq!(3, foo.0);
        assert_eq!(4, foo.1);

        let mut iter = patch.iter_fields();
        assert_eq!(3, *iter.next().unwrap().downcast_ref::<u32>().unwrap());
        assert_eq!(4, *iter.next().unwrap().downcast_ref::<u64>().unwrap());
    }

    #[test]
    #[should_panic(expected = "the given key does not support hashing")]
    fn reflect_map_no_hash() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
        }

        let foo = Foo { a: 1 };

        let mut map = DynamicMap::default();
        map.insert(foo, 10u32);
    }

    #[test]
    fn reflect_ignore() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
        }

        let foo = Foo { a: 1, _b: 2 };

        let values: Vec<u32> = foo
            .iter_fields()
            .map(|value| *value.downcast_ref::<u32>().unwrap())
            .collect();
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn should_call_from_reflect_dynamically() {
        #[derive(Reflect)]
        struct MyStruct {
            foo: usize,
        }

        // Register
        let mut registry = TypeRegistry::default();
        registry.register::<MyStruct>();

        // Get type data
        let type_id = TypeId::of::<MyStruct>();
        let rfr = registry
            .get_type_data::<ReflectFromReflect>(type_id)
            .expect("the FromReflect trait should be registered");

        // Call from_reflect
        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.insert("foo", 123usize);
        let reflected = rfr
            .from_reflect(&dynamic_struct)
            .expect("the type should be properly reflected");

        // Assert
        let expected = MyStruct { foo: 123 };
        assert!(expected
            .reflect_partial_eq(reflected.as_ref())
            .unwrap_or_default());
        let not_expected = MyStruct { foo: 321 };
        assert!(!not_expected
            .reflect_partial_eq(reflected.as_ref())
            .unwrap_or_default());
    }

    #[test]
    fn from_reflect_should_use_default_field_attributes() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        struct MyStruct {
            // Use `Default::default()`
            // Note that this isn't an ignored field
            #[reflect(default)]
            foo: String,

            // Use `get_bar_default()`
            #[reflect(ignore)]
            #[reflect(default = "get_bar_default")]
            bar: NotReflect,

            // Ensure attributes can be combined
            #[reflect(ignore, default = "get_bar_default")]
            baz: NotReflect,
        }

        #[derive(Eq, PartialEq, Debug)]
        struct NotReflect(usize);

        fn get_bar_default() -> NotReflect {
            NotReflect(123)
        }

        let expected = MyStruct {
            foo: String::default(),
            bar: NotReflect(123),
            baz: NotReflect(123),
        };

        let dyn_struct = DynamicStruct::default();
        let my_struct = <MyStruct as FromReflect>::from_reflect(&dyn_struct);

        assert_eq!(Some(expected), my_struct);
    }

    #[test]
    fn from_reflect_should_use_default_variant_field_attributes() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        enum MyEnum {
            Foo(#[reflect(default)] String),
            Bar {
                #[reflect(default = "get_baz_default")]
                #[reflect(ignore)]
                baz: usize,
            },
        }

        fn get_baz_default() -> usize {
            123
        }

        let expected = MyEnum::Foo(String::default());

        let dyn_enum = DynamicEnum::new("Foo", DynamicTuple::default());
        let my_enum = <MyEnum as FromReflect>::from_reflect(&dyn_enum);

        assert_eq!(Some(expected), my_enum);

        let expected = MyEnum::Bar {
            baz: get_baz_default(),
        };

        let dyn_enum = DynamicEnum::new("Bar", DynamicStruct::default());
        let my_enum = <MyEnum as FromReflect>::from_reflect(&dyn_enum);

        assert_eq!(Some(expected), my_enum);
    }

    #[test]
    fn from_reflect_should_use_default_container_attribute() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        #[reflect(Default)]
        struct MyStruct {
            foo: String,
            #[reflect(ignore)]
            bar: usize,
        }

        impl Default for MyStruct {
            fn default() -> Self {
                Self {
                    foo: String::from("Hello"),
                    bar: 123,
                }
            }
        }

        let expected = MyStruct {
            foo: String::from("Hello"),
            bar: 123,
        };

        let dyn_struct = DynamicStruct::default();
        let my_struct = <MyStruct as FromReflect>::from_reflect(&dyn_struct);

        assert_eq!(Some(expected), my_struct);
    }

    #[test]
    fn reflect_complex_patch() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        #[reflect(PartialEq)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
            c: Vec<isize>,
            d: HashMap<usize, i8>,
            e: Bar,
            f: (i32, Vec<isize>, Bar),
            g: Vec<(Baz, HashMap<usize, Bar>)>,
            h: [u32; 2],
        }

        #[derive(Reflect, Eq, PartialEq, Clone, Debug)]
        #[reflect(PartialEq)]
        struct Bar {
            x: u32,
        }

        #[derive(Reflect, Eq, PartialEq, Debug)]
        struct Baz(String);

        let mut hash_map = HashMap::default();
        hash_map.insert(1, 1);
        hash_map.insert(2, 2);

        let mut hash_map_baz = HashMap::default();
        hash_map_baz.insert(1, Bar { x: 0 });

        let mut foo = Foo {
            a: 1,
            _b: 1,
            c: vec![1, 2],
            d: hash_map,
            e: Bar { x: 1 },
            f: (1, vec![1, 2], Bar { x: 1 }),
            g: vec![(Baz("string".to_string()), hash_map_baz)],
            h: [2; 2],
        };

        let mut foo_patch = DynamicStruct::default();
        foo_patch.insert("a", 2u32);
        foo_patch.insert("b", 2u32); // this should be ignored

        let mut list = DynamicList::default();
        list.push(3isize);
        list.push(4isize);
        list.push(5isize);
        foo_patch.insert("c", list.clone_dynamic());

        let mut map = DynamicMap::default();
        map.insert(2usize, 3i8);
        map.insert(3usize, 4i8);
        foo_patch.insert("d", map);

        let mut bar_patch = DynamicStruct::default();
        bar_patch.insert("x", 2u32);
        foo_patch.insert("e", bar_patch.clone_dynamic());

        let mut tuple = DynamicTuple::default();
        tuple.insert(2i32);
        tuple.insert(list);
        tuple.insert(bar_patch);
        foo_patch.insert("f", tuple);

        let mut composite = DynamicList::default();
        composite.push({
            let mut tuple = DynamicTuple::default();
            tuple.insert({
                let mut tuple_struct = DynamicTupleStruct::default();
                tuple_struct.insert("new_string".to_string());
                tuple_struct
            });
            tuple.insert({
                let mut map = DynamicMap::default();
                map.insert(1usize, {
                    let mut struct_ = DynamicStruct::default();
                    struct_.insert("x", 7u32);
                    struct_
                });
                map
            });
            tuple
        });
        foo_patch.insert("g", composite);

        let array = DynamicArray::from_vec(vec![2u32, 2u32]);
        foo_patch.insert("h", array);

        foo.apply(&foo_patch);

        let mut hash_map = HashMap::default();
        hash_map.insert(1, 1);
        hash_map.insert(2, 3);
        hash_map.insert(3, 4);

        let mut hash_map_baz = HashMap::default();
        hash_map_baz.insert(1, Bar { x: 7 });

        let expected_foo = Foo {
            a: 2,
            _b: 1,
            c: vec![3, 4, 5],
            d: hash_map,
            e: Bar { x: 2 },
            f: (2, vec![3, 4, 5], Bar { x: 2 }),
            g: vec![(Baz("new_string".to_string()), hash_map_baz.clone())],
            h: [2; 2],
        };

        assert_eq!(foo, expected_foo);

        let new_foo = Foo::from_reflect(&foo_patch)
            .expect("error while creating a concrete type from a dynamic type");

        let mut hash_map = HashMap::default();
        hash_map.insert(2, 3);
        hash_map.insert(3, 4);

        let expected_new_foo = Foo {
            a: 2,
            _b: 0,
            c: vec![3, 4, 5],
            d: hash_map,
            e: Bar { x: 2 },
            f: (2, vec![3, 4, 5], Bar { x: 2 }),
            g: vec![(Baz("new_string".to_string()), hash_map_baz)],
            h: [2; 2],
        };

        assert_eq!(new_foo, expected_new_foo);
    }

    #[test]
    fn reflect_serialize() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
            c: Vec<isize>,
            d: HashMap<usize, i8>,
            e: Bar,
            f: String,
            g: (i32, Vec<isize>, Bar),
            h: [u32; 2],
        }

        #[derive(Reflect, Serialize, Deserialize)]
        #[reflect(Serialize, Deserialize)]
        struct Bar {
            x: u32,
        }

        let mut hash_map = HashMap::default();
        hash_map.insert(1, 1);
        hash_map.insert(2, 2);
        let foo = Foo {
            a: 1,
            _b: 1,
            c: vec![1, 2],
            d: hash_map,
            e: Bar { x: 1 },
            f: "hi".to_string(),
            g: (1, vec![1, 2], Bar { x: 1 }),
            h: [2; 2],
        };

        let mut registry = TypeRegistry::default();
        registry.register::<u32>();
        registry.register::<i8>();
        registry.register::<i32>();
        registry.register::<usize>();
        registry.register::<isize>();
        registry.register::<Foo>();
        registry.register::<Bar>();
        registry.register::<String>();
        registry.register::<Vec<isize>>();
        registry.register::<HashMap<usize, i8>>();
        registry.register::<(i32, Vec<isize>, Bar)>();
        registry.register::<[u32; 2]>();

        let serializer = ReflectSerializer::new(&foo, &registry);
        let serialized = to_string_pretty(&serializer, PrettyConfig::default()).unwrap();

        let mut deserializer = Deserializer::from_str(&serialized).unwrap();
        let reflect_deserializer = UntypedReflectDeserializer::new(&registry);
        let value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
        let dynamic_struct = value.take::<DynamicStruct>().unwrap();

        assert!(foo.reflect_partial_eq(&dynamic_struct).unwrap());
    }

    #[test]
    fn reflect_downcast() {
        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct Bar {
            y: u8,
        }

        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct Foo {
            x: i32,
            s: String,
            b: Bar,
            u: usize,
            t: ([f32; 3], String),
            v: Cow<'static, str>,
            w: Cow<'static, [u8]>,
        }

        let foo = Foo {
            x: 123,
            s: "String".to_string(),
            b: Bar { y: 255 },
            u: 1111111111111,
            t: ([3.0, 2.0, 1.0], "Tuple String".to_string()),
            v: Cow::Owned("Cow String".to_string()),
            w: Cow::Owned(vec![1, 2, 3]),
        };

        let foo2: Box<dyn Reflect> = Box::new(foo.clone());

        assert_eq!(foo, *foo2.downcast::<Foo>().unwrap());
    }

    #[test]
    fn should_drain_fields() {
        let array_value: Box<dyn Array> = Box::new([123_i32, 321_i32]);
        let fields = array_value.drain();
        assert!(fields[0].reflect_partial_eq(&123_i32).unwrap_or_default());
        assert!(fields[1].reflect_partial_eq(&321_i32).unwrap_or_default());

        let list_value: Box<dyn List> = Box::new(vec![123_i32, 321_i32]);
        let fields = list_value.drain();
        assert!(fields[0].reflect_partial_eq(&123_i32).unwrap_or_default());
        assert!(fields[1].reflect_partial_eq(&321_i32).unwrap_or_default());

        let tuple_value: Box<dyn Tuple> = Box::new((123_i32, 321_i32));
        let fields = tuple_value.drain();
        assert!(fields[0].reflect_partial_eq(&123_i32).unwrap_or_default());
        assert!(fields[1].reflect_partial_eq(&321_i32).unwrap_or_default());

        let map_value: Box<dyn Map> = Box::new(HashMap::from([(123_i32, 321_i32)]));
        let fields = map_value.drain();
        assert!(fields[0].0.reflect_partial_eq(&123_i32).unwrap_or_default());
        assert!(fields[0].1.reflect_partial_eq(&321_i32).unwrap_or_default());
    }

    #[test]
    fn reflect_take() {
        #[derive(Reflect, Debug, PartialEq)]
        #[reflect(PartialEq)]
        struct Bar {
            x: u32,
        }

        let x: Box<dyn Reflect> = Box::new(Bar { x: 2 });
        let y = x.take::<Bar>().unwrap();
        assert_eq!(y, Bar { x: 2 });
    }

    #[test]
    fn dynamic_names() {
        let list = Vec::<usize>::new();
        let dyn_list = list.clone_dynamic();
        assert_eq!(dyn_list.type_name(), std::any::type_name::<Vec<usize>>());

        let array = [b'0'; 4];
        let dyn_array = array.clone_dynamic();
        assert_eq!(dyn_array.type_name(), std::any::type_name::<[u8; 4]>());

        let map = HashMap::<usize, String>::default();
        let dyn_map = map.clone_dynamic();
        assert_eq!(
            dyn_map.type_name(),
            std::any::type_name::<HashMap<usize, String>>()
        );

        let tuple = (0usize, "1".to_string(), 2.0f32);
        let mut dyn_tuple = tuple.clone_dynamic();
        dyn_tuple.insert::<usize>(3);
        assert_eq!(
            dyn_tuple.type_name(),
            std::any::type_name::<(usize, String, f32, usize)>()
        );

        #[derive(Reflect)]
        struct TestStruct {
            a: usize,
        }
        let struct_ = TestStruct { a: 0 };
        let dyn_struct = struct_.clone_dynamic();
        assert_eq!(dyn_struct.type_name(), std::any::type_name::<TestStruct>());

        #[derive(Reflect)]
        struct TestTupleStruct(usize);
        let tuple_struct = TestTupleStruct(0);
        let dyn_tuple_struct = tuple_struct.clone_dynamic();
        assert_eq!(
            dyn_tuple_struct.type_name(),
            std::any::type_name::<TestTupleStruct>()
        );
    }

    #[test]
    fn reflect_type_path() {
        #[derive(TypePath)]
        struct Param;

        #[derive(TypePath)]
        struct Derive;

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        struct DerivePath;

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        #[type_name = "MyDerivePathName"]
        struct DerivePathName;

        #[derive(TypePath)]
        struct DeriveG<T>(PhantomData<T>);

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        struct DerivePathG<T, const N: usize>(PhantomData<T>);

        #[derive(TypePath)]
        #[type_path = "my_alias"]
        #[type_name = "MyDerivePathNameG"]
        struct DerivePathNameG<T>(PhantomData<T>);

        struct Macro;
        impl_type_path!((in my_alias) Macro);

        struct MacroName;
        impl_type_path!((in my_alias as MyMacroName) MacroName);

        struct MacroG<T, const N: usize>(PhantomData<T>);
        impl_type_path!((in my_alias) MacroG<T, const N: usize>);

        struct MacroNameG<T>(PhantomData<T>);
        impl_type_path!((in my_alias as MyMacroNameG) MacroNameG<T>);

        assert_eq!(Derive::type_path(), "bevy_reflect::tests::Derive");
        assert_eq!(DerivePath::type_path(), "my_alias::DerivePath");
        assert_eq!(DerivePathName::type_path(), "my_alias::MyDerivePathName");

        assert_eq!(
            DeriveG::<Param>::type_path(),
            "bevy_reflect::tests::DeriveG<bevy_reflect::tests::Param>"
        );
        assert_eq!(
            DerivePathG::<Param, 10>::type_path(),
            "my_alias::DerivePathG<bevy_reflect::tests::Param, 10>"
        );
        assert_eq!(
            DerivePathNameG::<Param>::type_path(),
            "my_alias::MyDerivePathNameG<bevy_reflect::tests::Param>"
        );

        assert_eq!(Macro::type_path(), "my_alias::Macro");
        assert_eq!(MacroName::type_path(), "my_alias::MyMacroName");
        assert_eq!(
            MacroG::<Param, 10>::type_path(),
            "my_alias::MacroG<bevy_reflect::tests::Param, 10>"
        );
        assert_eq!(
            MacroNameG::<Param>::type_path(),
            "my_alias::MyMacroNameG<bevy_reflect::tests::Param>"
        );

        assert_eq!(Derive::short_type_path(), "Derive");
        assert_eq!(DerivePath::short_type_path(), "DerivePath");
        assert_eq!(DerivePathName::short_type_path(), "MyDerivePathName");

        assert_eq!(DeriveG::<Param>::short_type_path(), "DeriveG<Param>");
        assert_eq!(
            DerivePathG::<Param, 10>::short_type_path(),
            "DerivePathG<Param, 10>"
        );
        assert_eq!(
            DerivePathNameG::<Param>::short_type_path(),
            "MyDerivePathNameG<Param>"
        );

        assert_eq!(Macro::short_type_path(), "Macro");
        assert_eq!(MacroName::short_type_path(), "MyMacroName");
        assert_eq!(MacroG::<Param, 10>::short_type_path(), "MacroG<Param, 10>");
        assert_eq!(
            MacroNameG::<Param>::short_type_path(),
            "MyMacroNameG<Param>"
        );
    }

    #[test]
    fn reflect_type_info() {
        // TypeInfo
        let info = i32::type_info();
        assert_eq!(std::any::type_name::<i32>(), info.type_name());
        assert_eq!(std::any::TypeId::of::<i32>(), info.type_id());

        // TypeInfo (unsized)
        assert_eq!(
            std::any::TypeId::of::<dyn Reflect>(),
            <dyn Reflect as Typed>::type_info().type_id()
        );

        // TypeInfo (instance)
        let value: &dyn Reflect = &123_i32;
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<i32>());

        // Struct
        #[derive(Reflect)]
        struct MyStruct {
            foo: i32,
            bar: usize,
        }

        let info = MyStruct::type_info();
        if let TypeInfo::Struct(info) = info {
            assert!(info.is::<MyStruct>());
            assert_eq!(std::any::type_name::<MyStruct>(), info.type_name());
            assert_eq!(
                std::any::type_name::<i32>(),
                info.field("foo").unwrap().type_name()
            );
            assert_eq!(
                std::any::TypeId::of::<i32>(),
                info.field("foo").unwrap().type_id()
            );
            assert!(info.field("foo").unwrap().is::<i32>());
            assert_eq!("foo", info.field("foo").unwrap().name());
            assert_eq!(
                std::any::type_name::<usize>(),
                info.field_at(1).unwrap().type_name()
            );
        } else {
            panic!("Expected `TypeInfo::Struct`");
        }

        let value: &dyn Reflect = &MyStruct { foo: 123, bar: 321 };
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyStruct>());

        // Struct (generic)
        #[derive(Reflect)]
        struct MyGenericStruct<T> {
            foo: T,
            bar: usize,
        }

        let info = <MyGenericStruct<i32>>::type_info();
        if let TypeInfo::Struct(info) = info {
            assert!(info.is::<MyGenericStruct<i32>>());
            assert_eq!(
                std::any::type_name::<MyGenericStruct<i32>>(),
                info.type_name()
            );
            assert_eq!(
                std::any::type_name::<i32>(),
                info.field("foo").unwrap().type_name()
            );
            assert_eq!("foo", info.field("foo").unwrap().name());
            assert_eq!(
                std::any::type_name::<usize>(),
                info.field_at(1).unwrap().type_name()
            );
        } else {
            panic!("Expected `TypeInfo::Struct`");
        }

        let value: &dyn Reflect = &MyGenericStruct {
            foo: String::from("Hello!"),
            bar: 321,
        };
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyGenericStruct<String>>());

        // Tuple Struct
        #[derive(Reflect)]
        struct MyTupleStruct(usize, i32, MyStruct);

        let info = MyTupleStruct::type_info();
        if let TypeInfo::TupleStruct(info) = info {
            assert!(info.is::<MyTupleStruct>());
            assert_eq!(std::any::type_name::<MyTupleStruct>(), info.type_name());
            assert_eq!(
                std::any::type_name::<i32>(),
                info.field_at(1).unwrap().type_name()
            );
            assert!(info.field_at(1).unwrap().is::<i32>());
        } else {
            panic!("Expected `TypeInfo::TupleStruct`");
        }

        // Tuple
        type MyTuple = (u32, f32, String);

        let info = MyTuple::type_info();
        if let TypeInfo::Tuple(info) = info {
            assert!(info.is::<MyTuple>());
            assert_eq!(std::any::type_name::<MyTuple>(), info.type_name());
            assert_eq!(
                std::any::type_name::<f32>(),
                info.field_at(1).unwrap().type_name()
            );
        } else {
            panic!("Expected `TypeInfo::Tuple`");
        }

        let value: &dyn Reflect = &(123_u32, 1.23_f32, String::from("Hello!"));
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyTuple>());

        // List
        type MyList = Vec<usize>;

        let info = MyList::type_info();
        if let TypeInfo::List(info) = info {
            assert!(info.is::<MyList>());
            assert!(info.item_is::<usize>());
            assert_eq!(std::any::type_name::<MyList>(), info.type_name());
            assert_eq!(std::any::type_name::<usize>(), info.item_type_name());
        } else {
            panic!("Expected `TypeInfo::List`");
        }

        let value: &dyn Reflect = &vec![123_usize];
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyList>());

        // List (SmallVec)
        #[cfg(feature = "smallvec")]
        {
            type MySmallVec = smallvec::SmallVec<[String; 2]>;

            let info = MySmallVec::type_info();
            if let TypeInfo::List(info) = info {
                assert!(info.is::<MySmallVec>());
                assert!(info.item_is::<String>());
                assert_eq!(std::any::type_name::<MySmallVec>(), info.type_name());
                assert_eq!(std::any::type_name::<String>(), info.item_type_name());
            } else {
                panic!("Expected `TypeInfo::List`");
            }

            let value: MySmallVec = smallvec::smallvec![String::default(); 2];
            let value: &dyn Reflect = &value;
            let info = value.get_represented_type_info().unwrap();
            assert!(info.is::<MySmallVec>());
        }

        // Array
        type MyArray = [usize; 3];

        let info = MyArray::type_info();
        if let TypeInfo::Array(info) = info {
            assert!(info.is::<MyArray>());
            assert!(info.item_is::<usize>());
            assert_eq!(std::any::type_name::<MyArray>(), info.type_name());
            assert_eq!(std::any::type_name::<usize>(), info.item_type_name());
            assert_eq!(3, info.capacity());
        } else {
            panic!("Expected `TypeInfo::Array`");
        }

        let value: &dyn Reflect = &[1usize, 2usize, 3usize];
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyArray>());

        // Cow<'static, str>
        type MyCowStr = Cow<'static, str>;

        let info = MyCowStr::type_info();
        if let TypeInfo::Value(info) = info {
            assert!(info.is::<MyCowStr>());
            assert_eq!(std::any::type_name::<MyCowStr>(), info.type_name());
        } else {
            panic!("Expected `TypeInfo::Value`");
        }

        let value: &dyn Reflect = &Cow::<'static, str>::Owned("Hello!".to_string());
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyCowStr>());

        // Cow<'static, [u8]>
        type MyCowSlice = Cow<'static, [u8]>;

        let info = MyCowSlice::type_info();
        if let TypeInfo::List(info) = info {
            assert!(info.is::<MyCowSlice>());
            assert!(info.item_is::<u8>());
            assert_eq!(std::any::type_name::<MyCowSlice>(), info.type_name());
            assert_eq!(std::any::type_name::<u8>(), info.item_type_name());
        } else {
            panic!("Expected `TypeInfo::List`");
        }

        let value: &dyn Reflect = &Cow::<'static, [u8]>::Owned(vec![0, 1, 2, 3]);
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyCowSlice>());

        // Map
        type MyMap = HashMap<usize, f32>;

        let info = MyMap::type_info();
        if let TypeInfo::Map(info) = info {
            assert!(info.is::<MyMap>());
            assert!(info.key_is::<usize>());
            assert!(info.value_is::<f32>());
            assert_eq!(std::any::type_name::<MyMap>(), info.type_name());
            assert_eq!(std::any::type_name::<usize>(), info.key_type_name());
            assert_eq!(std::any::type_name::<f32>(), info.value_type_name());
        } else {
            panic!("Expected `TypeInfo::Map`");
        }

        let value: &dyn Reflect = &MyMap::new();
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyMap>());

        // Value
        type MyValue = String;

        let info = MyValue::type_info();
        if let TypeInfo::Value(info) = info {
            assert!(info.is::<MyValue>());
            assert_eq!(std::any::type_name::<MyValue>(), info.type_name());
        } else {
            panic!("Expected `TypeInfo::Value`");
        }

        let value: &dyn Reflect = &String::from("Hello!");
        let info = value.get_represented_type_info().unwrap();
        assert!(info.is::<MyValue>());
    }

    #[test]
    fn should_permit_higher_ranked_lifetimes() {
        #[derive(Reflect)]
        #[reflect(from_reflect = false)]
        struct TestStruct {
            #[reflect(ignore)]
            _hrl: for<'a> fn(&'a str) -> &'a str,
        }

        impl Default for TestStruct {
            fn default() -> Self {
                TestStruct {
                    _hrl: |input| input,
                }
            }
        }

        fn get_type_registration<T: GetTypeRegistration>() {}
        get_type_registration::<TestStruct>();
    }

    #[test]
    fn should_permit_valid_represented_type_for_dynamic() {
        let type_info = <[i32; 2] as Typed>::type_info();
        let mut dynamic_array = [123; 2].clone_dynamic();
        dynamic_array.set_represented_type(Some(type_info));
    }

    #[test]
    #[should_panic(expected = "expected TypeInfo::Array but received")]
    fn should_prohibit_invalid_represented_type_for_dynamic() {
        let type_info = <(i32, i32) as Typed>::type_info();
        let mut dynamic_array = [123; 2].clone_dynamic();
        dynamic_array.set_represented_type(Some(type_info));
    }

    #[cfg(feature = "documentation")]
    mod docstrings {
        use super::*;

        #[test]
        fn should_not_contain_docs() {
            // Regular comments do not count as doc comments,
            // and are therefore not reflected.
            #[derive(Reflect)]
            struct SomeStruct;

            let info = <SomeStruct as Typed>::type_info();
            assert_eq!(None, info.docs());

            /*
             * Block comments do not count as doc comments,
             * and are therefore not reflected.
             */
            #[derive(Reflect)]
            struct SomeOtherStruct;

            let info = <SomeOtherStruct as Typed>::type_info();
            assert_eq!(None, info.docs());
        }

        #[test]
        fn should_contain_docs() {
            /// Some struct.
            ///
            /// # Example
            ///
            /// ```ignore
            /// let some_struct = SomeStruct;
            /// ```
            #[derive(Reflect)]
            struct SomeStruct;

            let info = <SomeStruct as Typed>::type_info();
            assert_eq!(
                Some(" Some struct.\n\n # Example\n\n ```ignore\n let some_struct = SomeStruct;\n ```"),
                info.docs()
            );

            #[doc = "The compiler automatically converts `///`-style comments into `#[doc]` attributes."]
            #[doc = "Of course, you _could_ use the attribute directly if you wanted to."]
            #[doc = "Both will be reflected."]
            #[derive(Reflect)]
            struct SomeOtherStruct;

            let info = <SomeOtherStruct as Typed>::type_info();
            assert_eq!(
                Some("The compiler automatically converts `///`-style comments into `#[doc]` attributes.\nOf course, you _could_ use the attribute directly if you wanted to.\nBoth will be reflected."),
                info.docs()
            );

            /// Some tuple struct.
            #[derive(Reflect)]
            struct SomeTupleStruct(usize);

            let info = <SomeTupleStruct as Typed>::type_info();
            assert_eq!(Some(" Some tuple struct."), info.docs());

            /// Some enum.
            #[derive(Reflect)]
            enum SomeEnum {
                Foo,
            }

            let info = <SomeEnum as Typed>::type_info();
            assert_eq!(Some(" Some enum."), info.docs());

            #[derive(Clone)]
            struct SomePrimitive;
            impl_reflect_value!(
                /// Some primitive for which we have attributed custom documentation.
                (in bevy_reflect::tests) SomePrimitive
            );

            let info = <SomePrimitive as Typed>::type_info();
            assert_eq!(
                Some(" Some primitive for which we have attributed custom documentation."),
                info.docs()
            );
        }

        #[test]
        fn fields_should_contain_docs() {
            #[derive(Reflect)]
            struct SomeStruct {
                /// The name
                name: String,
                /// The index
                index: usize,
                // Not documented...
                data: Vec<i32>,
            }

            let info = <SomeStruct as Typed>::type_info();
            if let TypeInfo::Struct(info) = info {
                let mut fields = info.iter();
                assert_eq!(Some(" The name"), fields.next().unwrap().docs());
                assert_eq!(Some(" The index"), fields.next().unwrap().docs());
                assert_eq!(None, fields.next().unwrap().docs());
            } else {
                panic!("expected struct info");
            }
        }

        #[test]
        fn variants_should_contain_docs() {
            #[derive(Reflect)]
            enum SomeEnum {
                // Not documented...
                Nothing,
                /// Option A
                A(
                    /// Index
                    usize,
                ),
                /// Option B
                B {
                    /// Name
                    name: String,
                },
            }

            let info = <SomeEnum as Typed>::type_info();
            if let TypeInfo::Enum(info) = info {
                let mut variants = info.iter();
                assert_eq!(None, variants.next().unwrap().docs());

                let variant = variants.next().unwrap();
                assert_eq!(Some(" Option A"), variant.docs());
                if let VariantInfo::Tuple(variant) = variant {
                    let field = variant.field_at(0).unwrap();
                    assert_eq!(Some(" Index"), field.docs());
                } else {
                    panic!("expected tuple variant")
                }

                let variant = variants.next().unwrap();
                assert_eq!(Some(" Option B"), variant.docs());
                if let VariantInfo::Struct(variant) = variant {
                    let field = variant.field_at(0).unwrap();
                    assert_eq!(Some(" Name"), field.docs());
                } else {
                    panic!("expected struct variant")
                }
            } else {
                panic!("expected enum info");
            }
        }
    }

    #[test]
    fn into_reflect() {
        trait TestTrait: Reflect {}

        #[derive(Reflect)]
        struct TestStruct;

        impl TestTrait for TestStruct {}

        let trait_object: Box<dyn TestTrait> = Box::new(TestStruct);

        // Should compile:
        let _ = trait_object.into_reflect();
    }

    #[test]
    fn as_reflect() {
        trait TestTrait: Reflect {}

        #[derive(Reflect)]
        struct TestStruct;

        impl TestTrait for TestStruct {}

        let trait_object: Box<dyn TestTrait> = Box::new(TestStruct);

        // Should compile:
        let _ = trait_object.as_reflect();
    }

    #[test]
    fn should_reflect_debug() {
        #[derive(Reflect)]
        struct Test {
            value: usize,
            list: Vec<String>,
            array: [f32; 3],
            map: HashMap<i32, f32>,
            a_struct: SomeStruct,
            a_tuple_struct: SomeTupleStruct,
            enum_unit: SomeEnum,
            enum_tuple: SomeEnum,
            enum_struct: SomeEnum,
            custom: CustomDebug,
            #[reflect(ignore)]
            #[allow(dead_code)]
            ignored: isize,
        }

        #[derive(Reflect)]
        struct SomeStruct {
            foo: String,
        }

        #[derive(Reflect)]
        enum SomeEnum {
            A,
            B(usize),
            C { value: i32 },
        }

        #[derive(Reflect)]
        struct SomeTupleStruct(String);

        #[derive(Reflect)]
        #[reflect(Debug)]
        struct CustomDebug;
        impl Debug for CustomDebug {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str("Cool debug!")
            }
        }

        let mut map = HashMap::new();
        map.insert(123, 1.23);

        let test = Test {
            value: 123,
            list: vec![String::from("A"), String::from("B"), String::from("C")],
            array: [1.0, 2.0, 3.0],
            map,
            a_struct: SomeStruct {
                foo: String::from("A Struct!"),
            },
            a_tuple_struct: SomeTupleStruct(String::from("A Tuple Struct!")),
            enum_unit: SomeEnum::A,
            enum_tuple: SomeEnum::B(123),
            enum_struct: SomeEnum::C { value: 321 },
            custom: CustomDebug,
            ignored: 321,
        };

        let reflected: &dyn Reflect = &test;
        let expected = r#"
bevy_reflect::tests::should_reflect_debug::Test {
    value: 123,
    list: [
        "A",
        "B",
        "C",
    ],
    array: [
        1.0,
        2.0,
        3.0,
    ],
    map: {
        123: 1.23,
    },
    a_struct: bevy_reflect::tests::should_reflect_debug::SomeStruct {
        foo: "A Struct!",
    },
    a_tuple_struct: bevy_reflect::tests::should_reflect_debug::SomeTupleStruct(
        "A Tuple Struct!",
    ),
    enum_unit: A,
    enum_tuple: B(
        123,
    ),
    enum_struct: C {
        value: 321,
    },
    custom: Cool debug!,
}"#;

        assert_eq!(expected, format!("\n{reflected:#?}"));
    }

    #[test]
    fn multiple_reflect_lists() {
        #[derive(Hash, PartialEq, Reflect)]
        #[reflect(Debug, Hash)]
        #[reflect(PartialEq)]
        struct Foo(i32);

        impl Debug for Foo {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "Foo")
            }
        }

        let foo = Foo(123);
        let foo: &dyn Reflect = &foo;

        assert!(foo.reflect_hash().is_some());
        assert_eq!(Some(true), foo.reflect_partial_eq(foo));
        assert_eq!("Foo".to_string(), format!("{foo:?}"));
    }

    #[test]
    fn multiple_reflect_value_lists() {
        #[derive(Clone, Hash, PartialEq, Reflect)]
        #[reflect_value(Debug, Hash)]
        #[reflect_value(PartialEq)]
        struct Foo(i32);

        impl Debug for Foo {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "Foo")
            }
        }

        let foo = Foo(123);
        let foo: &dyn Reflect = &foo;

        assert!(foo.reflect_hash().is_some());
        assert_eq!(Some(true), foo.reflect_partial_eq(foo));
        assert_eq!("Foo".to_string(), format!("{foo:?}"));
    }

    #[test]
    fn custom_debug_function() {
        #[derive(Reflect)]
        #[reflect(Debug(custom_debug))]
        struct Foo {
            a: u32,
        }

        fn custom_debug(_x: &Foo, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "123")
        }

        let foo = Foo { a: 1 };
        let foo: &dyn Reflect = &foo;

        assert_eq!("123", format!("{:?}", foo));
    }

    #[test]
    fn recursive_typed_storage_does_not_hang() {
        #[derive(Reflect)]
        struct Recurse<T>(T);

        let _ = <Recurse<Recurse<()>> as Typed>::type_info();
        let _ = <Recurse<Recurse<()>> as TypePath>::type_path();
    }

    #[test]
    fn can_opt_out_type_path() {
        #[derive(Reflect)]
        #[reflect(type_path = false)]
        struct Foo<T> {
            #[reflect(ignore)]
            _marker: PhantomData<T>,
        }

        struct NotTypePath;

        impl<T: 'static> TypePath for Foo<T> {
            fn type_path() -> &'static str {
                std::any::type_name::<Self>()
            }

            fn short_type_path() -> &'static str {
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    bevy_utils::get_short_name(std::any::type_name::<Self>())
                })
            }

            fn crate_name() -> Option<&'static str> {
                Some("bevy_reflect")
            }

            fn module_path() -> Option<&'static str> {
                Some("bevy_reflect::tests")
            }

            fn type_ident() -> Option<&'static str> {
                Some("Foo")
            }
        }

        // Can use `TypePath`
        let path = <Foo<NotTypePath> as TypePath>::type_path();
        assert_eq!("bevy_reflect::tests::can_opt_out_type_path::Foo<bevy_reflect::tests::can_opt_out_type_path::NotTypePath>", path);

        // Can register the type
        let mut registry = TypeRegistry::default();
        registry.register::<Foo<NotTypePath>>();

        let registration = registry.get(TypeId::of::<Foo<NotTypePath>>()).unwrap();
        assert_eq!("Foo<NotTypePath>", registration.short_name());
    }

    #[cfg(feature = "glam")]
    mod glam {
        use super::*;

        #[test]
        fn vec3_serialization() {
            let v = vec3(12.0, 3.0, -6.9);

            let mut registry = TypeRegistry::default();
            registry.register::<f32>();
            registry.register::<Vec3>();

            let ser = ReflectSerializer::new(&v, &registry);

            let config = PrettyConfig::default()
                .new_line(String::from("\n"))
                .indentor(String::from("    "));
            let output = to_string_pretty(&ser, config).unwrap();
            let expected = r#"
{
    "glam::f32::vec3::Vec3": (
        x: 12.0,
        y: 3.0,
        z: -6.9,
    ),
}"#;

            assert_eq!(expected, format!("\n{output}"));
        }

        #[test]
        fn vec3_deserialization() {
            let data = r#"
{
    "glam::f32::vec3::Vec3": (
        x: 12.0,
        y: 3.0,
        z: -6.9,
    ),
}"#;

            let mut registry = TypeRegistry::default();
            registry.add_registration(Vec3::get_type_registration());
            registry.add_registration(f32::get_type_registration());

            let de = UntypedReflectDeserializer::new(&registry);

            let mut deserializer =
                ron::de::Deserializer::from_str(data).expect("Failed to acquire deserializer");

            let dynamic_struct = de
                .deserialize(&mut deserializer)
                .expect("Failed to deserialize");

            let mut result = Vec3::default();

            result.apply(&*dynamic_struct);

            assert_eq!(result, vec3(12.0, 3.0, -6.9));
        }

        #[test]
        fn vec3_field_access() {
            let mut v = vec3(1.0, 2.0, 3.0);

            assert_eq!(*v.get_field::<f32>("x").unwrap(), 1.0);

            *v.get_field_mut::<f32>("y").unwrap() = 6.0;

            assert_eq!(v.y, 6.0);
        }

        #[test]
        fn vec3_path_access() {
            let mut v = vec3(1.0, 2.0, 3.0);

            assert_eq!(
                *v.reflect_path("x").unwrap().downcast_ref::<f32>().unwrap(),
                1.0
            );

            *v.reflect_path_mut("y")
                .unwrap()
                .downcast_mut::<f32>()
                .unwrap() = 6.0;

            assert_eq!(v.y, 6.0);
        }

        #[test]
        fn vec3_apply_dynamic() {
            let mut v = vec3(3.0, 3.0, 3.0);

            let mut d = DynamicStruct::default();
            d.insert("x", 4.0f32);
            d.insert("y", 2.0f32);
            d.insert("z", 1.0f32);

            v.apply(&d);

            assert_eq!(v, vec3(4.0, 2.0, 1.0));
        }
    }
}
