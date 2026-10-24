//! Rust reflection.
//!
//! # Features
//!
//! ## `auto_register_inventory`/`auto_register_static`
//!
//! | Default | Dependencies                                    |
//! | :-----: | :---------------------------------------------: |
//! | ✅      | `bevy_reflect2_macros/auto_register_inventory`  |
//! | ❌      | `bevy_reflect2_macros/auto_register_static`     |
//!
//! These features enable automatic registration of types that derive [`Reflect`].
//!
//! - `auto_register_inventory` uses `inventory` to collect types on supported platforms (Linux, macOS, iOS, FreeBSD, Android, Windows, WebAssembly).
//! - `auto_register_static` uses a platform-independent way to collect types, but requires additional setup and might
//!   slow down compilation, so it should only be used on platforms not supported by `inventory`.
//!   See the documentation for the [`load_type_registrations`] macro for more info.
//!
//! When enabled all types that derive [`Reflect`] on app startup are collected,
//! and [`TypeRegistry::register_derived_types`] can be used to register them at any point in the program.
//! However, this does not apply to types with generics: their desired monomorphized representations must be registered manually.

#![expect(unsafe_code, reason = "This crate uses unsafe")]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod impls;

/// The reflect prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::Reflect;
}

#[cfg(feature = "auto_register")]
mod auto_register;
mod builtin;
#[cfg(doctest)]
mod compile_fail;
mod kind;
mod reflect;
mod ty;
mod type_path;
mod type_registry;

pub use kind::*;
pub use reflect::*;
pub use ty::*;
pub use type_path::*;
pub use type_registry::*;

pub use bevy_reflect2_macros::{impl_reflect, impl_reflect_primitive, load_type_registrations};

#[doc(hidden)]
pub mod __macro_exports {
    pub use alloc::{boxed::Box, format};
    pub use bevy_ptr::Ptr;
    pub use bevy_utils::prelude::ShortName;
    use core::marker::PhantomData;

    /// Adapts a custom `Clone` function to the signature used by [`clone_with`](crate::clone_with).
    /// Taking `func` as an argument lets closures infer their parameter types.
    pub fn custom_clone<T>(func: impl FnOnce(&T) -> T) -> impl FnOnce(&T) -> Option<T> {
        move |value| Some(func(value))
    }

    /// Calls a custom `Default` function. Taking `func` as an argument lets closures infer their
    /// return types, and avoids calling closures where they are declared.
    pub fn custom_default<T>(func: impl FnOnce() -> T) -> T {
        func()
    }

    /// A marker that is invariant in `T`, so an `Invariant<A>` can only be assigned to an
    /// `Invariant<B>` if `A` and `B` are exactly the same type. A covariant marker (ex:
    /// `PhantomData<T>`) would also accept subtypes (ex: `for<'a> fn(&'a u8)` for
    /// `fn(&'static u8)`).
    pub struct Invariant<T: ?Sized>(PhantomData<*mut T>);

    /// Returns an [`Invariant`] of the type of `value`. Used to verify field types without
    /// coercions or subtyping.
    pub fn invariant_of<T: ?Sized>(_value: &T) -> Invariant<T> {
        Invariant(PhantomData)
    }

    #[cfg(feature = "auto_register")]
    pub mod auto_register {
        pub use crate::auto_register::*;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_reflect2, CreateTypeData, FieldKind, Kind, KindRef, Reflect, StructKind,
        TypeRegistry, Typed,
    };
    use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
    use core::{
        alloc::Layout,
        any::{Any, TypeId},
        cmp::Ordering,
        fmt,
    };

    #[test]
    fn reflect_struct_named() {
        #[derive(Reflect)]
        struct Foo {
            value: u32,
            other: f32,
        }

        let mut foo = Foo {
            value: 10,
            other: 20.,
        };
        let reflect = &mut foo as &mut dyn Reflect;
        assert_eq!(reflect.ty().type_id(), TypeId::of::<Foo>());
        assert_eq!(reflect.ty().type_path().full(), "bevy_reflect2::tests::Foo");
        assert_eq!(reflect.ty().type_path().short(), "Foo");
        assert_eq!(
            reflect.ty().type_path().module_path(),
            Some("bevy_reflect2::tests")
        );
        assert_eq!(reflect.ty().type_path().crate_name(), Some("bevy_reflect2"));

        // Non-generic type paths are readable in const contexts
        const FULL: Option<&str> = Foo::TYPE.type_path().try_full();
        const SHORT: Option<&str> = Foo::TYPE.type_path().try_short();
        const TYPE_IDENT: Option<&str> = Foo::TYPE.type_path().try_type_ident();
        const CRATE_NAME: Option<Option<&str>> = Foo::TYPE.type_path().try_crate_name();
        const MODULE_PATH: Option<Option<&str>> = Foo::TYPE.type_path().try_module_path();
        assert_eq!(FULL, Some("bevy_reflect2::tests::Foo"));
        assert_eq!(SHORT, Some("Foo"));
        assert_eq!(TYPE_IDENT, Some("Foo"));
        assert_eq!(CRATE_NAME, Some(Some("bevy_reflect2")));
        assert_eq!(MODULE_PATH, Some(Some("bevy_reflect2::tests")));

        let Kind::Struct(struct_kind) = reflect.ty().kind() else {
            panic!("Expected struct kind");
        };
        assert_eq!(*struct_kind.field_kind(), FieldKind::Named);
        assert_eq!(struct_kind.struct_kind(), StructKind::Struct);

        {
            let struct_ref = reflect.struct_ref().unwrap();
            let field = struct_ref.field(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<u32>());
            let value = field.downcast_ref::<u32>().unwrap();
            assert_eq!(*value, 10);

            let field = struct_ref.field(1).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<f32>());
            let value = field.downcast_ref::<f32>().unwrap();
            assert_eq!(*value, 20.);
        }

        {
            let mut struct_mut = reflect.struct_mut().unwrap();
            let field = struct_mut.field_mut(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<u32>());
            let value = field.downcast_mut::<u32>().unwrap();
            *value = 11;
        }
        assert_eq!(foo.value, 11);
    }

    #[test]
    fn reflect_struct_tuple() {
        #[derive(Reflect)]
        struct Foo(u32, f32);

        let mut foo = Foo(10, 20.);
        let reflect = &mut foo as &mut dyn Reflect;
        assert_eq!(reflect.ty().type_id(), TypeId::of::<Foo>());
        assert_eq!(reflect.ty().type_path().full(), "bevy_reflect2::tests::Foo");
        assert_eq!(reflect.ty().type_path().short(), "Foo");
        assert_eq!(
            reflect.ty().type_path().module_path(),
            Some("bevy_reflect2::tests")
        );
        assert_eq!(reflect.ty().type_path().crate_name(), Some("bevy_reflect2"));
        let Kind::Struct(struct_kind) = reflect.ty().kind() else {
            panic!("Expected struct kind");
        };
        assert_eq!(*struct_kind.field_kind(), FieldKind::Unnamed);
        assert_eq!(struct_kind.struct_kind(), StructKind::Struct);

        {
            let struct_ref = reflect.struct_ref().unwrap();
            let field = struct_ref.field(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<u32>());
            let value = field.downcast_ref::<u32>().unwrap();
            assert_eq!(*value, 10);

            let field = struct_ref.field(1).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<f32>());
            let value = field.downcast_ref::<f32>().unwrap();
            assert_eq!(*value, 20.);
        }

        {
            let mut struct_mut = reflect.struct_mut().unwrap();
            let field = struct_mut.field_mut(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<u32>());
            let value = field.downcast_mut::<u32>().unwrap();
            *value = 11;
        }
        assert_eq!(foo.0, 11);
    }

    #[test]
    fn reflect_struct_unit() {
        #[derive(Reflect)]
        struct Foo;

        let mut foo = Foo;
        let reflect = &mut foo as &mut dyn Reflect;
        assert_eq!(reflect.ty().type_id(), TypeId::of::<Foo>());
        assert_eq!(reflect.ty().type_path().full(), "bevy_reflect2::tests::Foo");
        assert_eq!(reflect.ty().type_path().short(), "Foo");
        assert_eq!(
            reflect.ty().type_path().module_path(),
            Some("bevy_reflect2::tests")
        );
        assert_eq!(reflect.ty().type_path().crate_name(), Some("bevy_reflect2"));
        let Kind::Struct(struct_kind) = reflect.ty().kind() else {
            panic!("Expected struct kind");
        };
        assert_eq!(*struct_kind.field_kind(), FieldKind::Unit);
        assert!(struct_kind.fields().is_empty());

        let struct_ref = reflect.struct_ref().unwrap();
        assert!(struct_ref.field(0).is_none());
    }

    #[test]
    fn reflect_enum() {
        #[derive(Reflect, PartialEq, Eq, Debug)]
        enum Foo {
            X { value: u32 },
            Y(u64),
            Z,
        }

        let mut foo = Foo::X { value: 1 };
        let reflect = &mut foo as &mut dyn Reflect;
        assert_eq!(reflect.ty().type_id(), TypeId::of::<Foo>());
        assert_eq!(reflect.ty().type_path().full(), "bevy_reflect2::tests::Foo");
        let Kind::Enum(_enum_kind) = reflect.ty().kind() else {
            panic!("Expected enum kind");
        };

        {
            let enum_ref = reflect.enum_ref().unwrap();
            let field = enum_ref.field(0).unwrap();
            assert_eq!(enum_ref.variant(), 0);
            assert_eq!(field.ty().type_id(), TypeId::of::<u32>());
            let value = field.downcast_ref::<u32>().unwrap();
            assert_eq!(*value, 1);
            assert!(enum_ref.field(1).is_none());
        }
        {
            let mut enum_mut = reflect.enum_mut().unwrap();
            assert_eq!(enum_mut.variant(), 0);
            let field = enum_mut.field_mut(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<u32>());
            let value = field.downcast_mut::<u32>().unwrap();
            *value = 11;
        }

        assert_eq!(foo, Foo::X { value: 11 });

        let mut foo = Foo::Y(1);
        let reflect = &mut foo as &mut dyn Reflect;

        {
            let enum_ref = reflect.enum_ref().unwrap();
            assert_eq!(enum_ref.variant(), 1);
            let field = enum_ref.field(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<u64>());
            let value = field.downcast_ref::<u64>().unwrap();
            assert_eq!(*value, 1);
            assert!(enum_ref.field(1).is_none());
        }
        {
            let mut enum_mut = reflect.enum_mut().unwrap();
            assert_eq!(enum_mut.variant(), 1);
            let field = enum_mut.field_mut(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<u64>());
            let value = field.downcast_mut::<u64>().unwrap();
            *value = 11;
        }
        assert_eq!(foo, Foo::Y(11));

        let foo = Foo::Z;
        let reflect = &foo as &dyn Reflect;

        let enum_ref = reflect.enum_ref().unwrap();
        assert_eq!(enum_ref.variant(), 2);
        assert!(enum_ref.field(0).is_none());
    }

    #[test]
    fn reflect_generics() {
        #[derive(Reflect)]
        struct Foo<T> {
            value: T,
            other: f32,
        }
        let foo = Foo {
            value: 10.0f32,
            other: 20.,
        };
        let reflect = &foo as &dyn Reflect;
        {
            assert_eq!(reflect.ty().type_id(), TypeId::of::<Foo::<f32>>());
            let struct_ref = reflect.struct_ref().unwrap();
            let field = struct_ref.field(0).unwrap();
            assert_eq!(field.ty().type_id(), TypeId::of::<f32>());
            let value = field.downcast_ref::<f32>().unwrap();
            assert_eq!(*value, 10.0f32);
        }
        assert_eq!(
            reflect.ty().type_path().full(),
            "bevy_reflect2::tests::Foo<f32>"
        );
        assert_eq!(reflect.ty().type_path().short(), "Foo<f32>");
        assert_eq!(reflect.ty().type_path().type_ident(), "Foo");
        assert_eq!(reflect.ty().type_path().crate_name(), Some("bevy_reflect2"));
        assert_eq!(
            Foo::<u32>::TYPE.type_path().full(),
            "bevy_reflect2::tests::Foo<u32>"
        );
        assert_eq!(Foo::<Foo<u32>>::TYPE.type_path().short(), "Foo<Foo<u32>>");
        // Generic type paths are only known at runtime
        assert_eq!(Foo::<u32>::TYPE.type_path().try_full(), None);
        assert_eq!(Foo::<u32>::TYPE.type_path().try_crate_name(), None);

        #[derive(Reflect)]
        enum Bar<T> {
            Some(T),
        }
        let bar = Bar::Some(10.0f32);
        let reflect = &bar as &dyn Reflect;
        {
            let enum_ref = reflect.enum_ref().unwrap();
            let field = enum_ref.field(0).unwrap();
            assert_eq!(enum_ref.variant(), 0);
            assert_eq!(field.ty().type_id(), TypeId::of::<f32>());
            let value = field.downcast_ref::<f32>().unwrap();
            assert_eq!(*value, 10.0);
        }
    }

    #[test]
    fn reflect_default() {
        #[derive(Reflect)]
        #[reflect(Default)]
        struct Foo(u32);

        impl Default for Foo {
            fn default() -> Self {
                Self(42)
            }
        }

        let foo = Foo::TYPE.default().unwrap();
        assert_eq!(foo.downcast_ref::<Foo>().unwrap().0, 42);

        #[derive(Reflect)]
        #[reflect(Default(bar))]
        struct Bar(u32);

        fn bar() -> Bar {
            Bar(10)
        }

        let bar = Bar::TYPE.default().unwrap();
        assert_eq!(bar.downcast_ref::<Bar>().unwrap().0, 10);

        #[derive(Reflect)]
        #[reflect(Default(baz))]
        struct Baz<T: Default>(T);

        fn baz<T: Default>() -> Baz<T> {
            Baz(T::default())
        }

        let baz = Baz::<u32>::TYPE.default().unwrap();
        assert_eq!(baz.downcast_ref::<Baz<u32>>().unwrap().0, 0);

        #[derive(Reflect)]
        struct NoDefault;
        assert!(NoDefault::TYPE.default().is_none());
    }

    #[test]
    fn reflect_default_drop_not_called() {
        #[derive(Reflect, Default)]
        #[reflect(Default)]
        struct Foo(u32);

        impl Drop for Foo {
            fn drop(&mut self) {
                panic!();
            }
        }

        let foo = Foo::TYPE.default().unwrap();
        let raw = Box::into_raw(foo);
        // SAFETY: `raw` was allocated with the layout of `Foo`. Deallocating skips `Foo`'s drop.
        unsafe {
            alloc::alloc::dealloc(raw.cast::<u8>(), Layout::new::<Foo>());
        }
    }

    #[test]
    fn reflect_default_custom_drop_not_called() {
        #[derive(Reflect)]
        #[reflect(Default(foo))]
        struct Foo(u32);

        impl Drop for Foo {
            fn drop(&mut self) {
                panic!();
            }
        }

        fn foo() -> Foo {
            Foo(1)
        }

        let foo = Foo::TYPE.default().unwrap();
        let raw = Box::into_raw(foo);
        // SAFETY: `raw` was allocated with the layout of `Foo`. Deallocating skips `Foo`'s drop.
        unsafe {
            alloc::alloc::dealloc(raw.cast::<u8>(), Layout::new::<Foo>());
        }
    }

    #[test]
    fn reflect_default_zero_sized() {
        #[derive(Reflect, Default, PartialEq, Eq, Debug)]
        #[reflect(Default)]
        struct Foo;

        let foo = Foo::TYPE.default().unwrap();
        assert_eq!(foo.downcast_ref::<Foo>(), Some(&Foo));

        let unit = <()>::TYPE.default().unwrap();
        assert!(unit.downcast_ref::<()>().is_some());
    }

    #[test]
    fn reflect_empty_enum() {
        #[derive(Reflect)]
        #[reflect(Debug, Clone, PartialEq, PartialOrd, Hash)]
        #[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
        enum Empty {}

        assert_eq!(Empty::TYPE.type_id(), TypeId::of::<Empty>());
        assert_eq!(
            Empty::TYPE.type_path().full(),
            "bevy_reflect2::tests::Empty"
        );
        let Kind::Enum(enum_kind) = Empty::TYPE.kind() else {
            panic!("Expected enum kind");
        };
        assert!(enum_kind.variants().is_empty());

        let mut type_registry = TypeRegistry::new();
        type_registry.register::<Empty>();
        assert!(type_registry.contains(TypeId::of::<Empty>()));
    }

    #[test]
    fn reflect_enum_field_names() {
        #[derive(Reflect)]
        enum Foo {
            X { value: u32 },
            Y(u64, u32),
        }

        let Kind::Enum(enum_kind) = Foo::TYPE.kind() else {
            panic!("Expected enum kind");
        };
        assert_eq!(enum_kind.variants()[0].fields[0].name, "value");
        assert_eq!(enum_kind.variants()[1].fields[0].name, "0");
        assert_eq!(enum_kind.variants()[1].fields[1].name, "1");

        let foo = Foo::Y(1, 2);
        let enum_ref = (&foo as &dyn Reflect).enum_ref().unwrap();
        assert_eq!(enum_ref.variant_type().name, "Y");
    }

    #[test]
    fn field_by_name() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _ignored: u32,
            b: f32,
        }

        #[derive(Reflect)]
        struct Tuple(u32, f32);

        let mut foo = Foo {
            a: 1,
            _ignored: 2,
            b: 3.0,
        };
        let reflect = &mut foo as &mut dyn Reflect;
        let struct_ref = reflect.struct_ref().unwrap();
        assert_eq!(struct_ref.struct_type().field_index("b"), Some(1));
        assert_eq!(
            struct_ref.field_by_name("a").unwrap().downcast_ref(),
            Some(&1u32)
        );
        assert_eq!(
            struct_ref.field_by_name("b").unwrap().downcast_ref(),
            Some(&3.0f32)
        );
        // Ignored fields are not reflected
        assert!(struct_ref.field_by_name("_ignored").is_none());
        assert!(struct_ref.field_by_name("missing").is_none());

        let mut struct_mut = reflect.struct_mut().unwrap();
        assert_eq!(
            struct_mut.field_by_name("a").unwrap().downcast_ref(),
            Some(&1u32)
        );
        *struct_mut
            .field_by_name_mut("b")
            .unwrap()
            .downcast_mut::<f32>()
            .unwrap() = 4.0;
        assert!(struct_mut.field_by_name_mut("missing").is_none());
        assert_eq!(foo.b, 4.0);

        // Unnamed fields are named by their index
        let tuple = Tuple(5, 6.0);
        let struct_ref = (&tuple as &dyn Reflect).struct_ref().unwrap();
        assert_eq!(
            struct_ref.field_by_name("1").unwrap().downcast_ref(),
            Some(&6.0f32)
        );
    }

    #[test]
    fn enum_field_by_name() {
        #[derive(Reflect)]
        enum Foo {
            X { a: u32, b: f32 },
            Y(u64),
        }

        let mut foo = Foo::X { a: 1, b: 2.0 };
        let reflect = &mut foo as &mut dyn Reflect;
        let enum_ref = reflect.enum_ref().unwrap();
        assert_eq!(enum_ref.variant_type().field_index("b"), Some(1));
        assert_eq!(
            enum_ref.field_by_name("b").unwrap().downcast_ref(),
            Some(&2.0f32)
        );
        // Only fields of the active variant are found
        assert!(enum_ref.field_by_name("0").is_none());

        let mut enum_mut = reflect.enum_mut().unwrap();
        assert_eq!(
            enum_mut.field_by_name("a").unwrap().downcast_ref(),
            Some(&1u32)
        );
        *enum_mut
            .field_by_name_mut("a")
            .unwrap()
            .downcast_mut::<u32>()
            .unwrap() = 3;
        assert!(enum_mut.field_by_name_mut("missing").is_none());
        let Foo::X { a, .. } = foo else {
            panic!("Expected X");
        };
        assert_eq!(a, 3);
    }

    #[test]
    fn construct_enum_variant() {
        #[derive(Reflect, Debug, PartialEq)]
        enum Foo {
            X {
                a: u32,
                #[reflect(ignore, default)]
                ignored: u32,
                b: String,
            },
            Y(u64, #[reflect(ignore, default(|| 7))] u8),
            Z,
            #[expect(
                dead_code,
                reason = "variants with ignored fields that have no default can't be constructed"
            )]
            NoDefault(#[reflect(ignore)] u32),
        }

        let Kind::Enum(enum_type) = Foo::TYPE.kind() else {
            panic!("Expected enum kind");
        };

        // Struct variant, with values provided by field name
        let mut values = vec![
            ("a", Box::new(1u32) as Box<dyn Reflect>),
            ("b", Box::new(String::from("b"))),
        ];
        let variant = enum_type.variant_index("X").unwrap();
        let foo = enum_type
            .construct_variant(variant, |_, field| {
                let index = values.iter().position(|(name, _)| *name == field.name)?;
                Some(values.remove(index).1)
            })
            .unwrap();
        assert_eq!(
            foo.take::<Foo>().ok(),
            Some(Foo::X {
                a: 1,
                ignored: 0,
                b: String::from("b")
            })
        );

        // Tuple variant, with a custom default for the ignored field
        let foo = enum_type
            .construct_variant(1, |index, _| {
                assert_eq!(index, 0);
                Some(Box::new(2u64))
            })
            .unwrap();
        assert_eq!(foo.take::<Foo>().ok(), Some(Foo::Y(2, 7)));

        // Unit variant
        let foo = enum_type
            .construct_variant(2, |_, _| unreachable!("unit variants have no fields"))
            .unwrap();
        assert_eq!(foo.take::<Foo>().ok(), Some(Foo::Z));

        // Values with the wrong type, missing values, missing variants, and ignored fields
        // without a default
        assert!(enum_type
            .construct_variant(1, |_, _| Some(Box::new(2u32)))
            .is_none());
        assert!(enum_type.construct_variant(1, |_, _| None).is_none());
        assert!(enum_type.construct_variant(4, |_, _| None).is_none());
        assert!(enum_type.variant_index("Missing").is_none());
        assert!(enum_type
            .construct_variant(3, |_, _| unreachable!("the variant can't be constructed"))
            .is_none());
    }

    #[test]
    fn construct_foreign_enum_variant() {
        let Kind::Enum(enum_type) = Option::<u32>::TYPE.kind() else {
            panic!("Expected enum kind");
        };
        let some = enum_type
            .construct_variant(enum_type.variant_index("Some").unwrap(), |_, _| {
                Some(Box::new(1u32))
            })
            .unwrap();
        assert_eq!(some.take::<Option<u32>>().ok(), Some(Some(1)));
        let none = enum_type
            .construct_variant(enum_type.variant_index("None").unwrap(), |_, _| None)
            .unwrap();
        assert_eq!(none.take::<Option<u32>>().ok(), Some(None));
    }

    #[test]
    fn custom_type_data() {
        #[derive(Clone)]
        struct ReflectThing(TypeId);

        impl<T: Any> CreateTypeData<T> for ReflectThing {
            fn create_type_data(_input: ()) -> Self {
                Self(TypeId::of::<T>())
            }
        }

        #[derive(Reflect)]
        #[reflect(Thing)]
        struct Foo;

        let mut type_registry = TypeRegistry::new();
        type_registry.register::<Foo>();
        let registration = type_registry.get(TypeId::of::<Foo>()).unwrap();
        let thing = registration.data::<ReflectThing>().unwrap();
        assert_eq!(thing.0, TypeId::of::<Foo>());
    }

    #[test]
    fn cfg_type_data() {
        #[derive(Clone)]
        struct ReflectEnabled;

        impl<T> CreateTypeData<T> for ReflectEnabled {
            fn create_type_data(_input: ()) -> Self {
                Self
            }
        }

        #[derive(Clone)]
        struct ReflectDisabled;

        impl<T> CreateTypeData<T> for ReflectDisabled {
            fn create_type_data(_input: ()) -> Self {
                Self
            }
        }

        #[derive(Reflect)]
        #[reflect(#[cfg(all())] Enabled, #[cfg(any())] Disabled)]
        struct Foo;

        let mut type_registry = TypeRegistry::new();
        type_registry.register::<Foo>();
        let registration = type_registry.get(TypeId::of::<Foo>()).unwrap();
        assert!(registration.data::<ReflectEnabled>().is_some());
        assert!(registration.data::<ReflectDisabled>().is_none());
    }

    #[test]
    fn initialize() {
        #[derive(Reflect, PartialEq, Eq, Debug)]
        struct Foo {
            a: u32,
            b: u32,
        }

        let mut values = [("a", 10), ("b", 20)].iter();

        // SAFETY: every field of `Foo` is a `u32`, and each one is initialized
        let value = unsafe {
            Foo::TYPE.initialize_struct(|field, ptr| {
                let (name, value) = values.next().unwrap();
                assert_eq!(*name, field.name);
                ptr.as_ptr().cast::<u32>().write(*value);
            })
        }
        .unwrap();

        assert_eq!(value.downcast_ref::<Foo>().unwrap(), &Foo { a: 10, b: 20 });
    }

    #[test]
    fn structural_builtins() {
        #[derive(Reflect)]
        struct Named {
            a: u32,
            b: String,
        }

        #[derive(Reflect)]
        struct Unnamed(u32, f32);

        #[derive(Reflect)]
        struct Unit;

        #[derive(Reflect)]
        enum Enum {
            X { value: u32 },
            Y(u64),
            Z,
        }

        let named = Named {
            a: 1,
            b: String::from("b"),
        };
        let reflect = &named as &dyn Reflect;
        assert_eq!(format!("{reflect:?}"), "Named { a: 1, b: \"b\" }");
        assert_eq!(
            format!("{reflect:#?}"),
            "Named {\n    a: 1,\n    b: \"b\",\n}"
        );
        let other = Named {
            a: 2,
            b: String::from("a"),
        };
        assert_eq!(reflect.reflect_partial_eq(&other), Some(false));
        assert_eq!(reflect.reflect_partial_cmp(&other), Some(Ordering::Less));
        assert_eq!(
            reflect.reflect_hash(),
            (&Named {
                a: 1,
                b: String::from("b"),
            } as &dyn Reflect)
                .reflect_hash()
        );
        assert_ne!(
            reflect.reflect_hash(),
            (&other as &dyn Reflect).reflect_hash()
        );
        let clone = reflect.reflect_clone().unwrap();
        assert_eq!(clone.reflect_partial_eq(reflect), Some(true));
        assert_eq!(clone.downcast_ref::<Named>().unwrap().b, "b");

        let unnamed = &Unnamed(1, 2.5) as &dyn Reflect;
        assert_eq!(format!("{unnamed:?}"), "Unnamed(1, 2.5)");
        assert_eq!(unnamed.reflect_partial_eq(&Unnamed(1, 2.5)), Some(true));
        assert_eq!(format!("{:?}", &Unit as &dyn Reflect), "Unit");
        assert_eq!(
            (&Unit as &dyn Reflect).reflect_partial_eq(&Unit),
            Some(true)
        );

        let x = &Enum::X { value: 1 } as &dyn Reflect;
        assert_eq!(format!("{x:?}"), "X { value: 1 }");
        assert_eq!(format!("{:?}", &Enum::Y(2) as &dyn Reflect), "Y(2)");
        assert_eq!(format!("{:?}", &Enum::Z as &dyn Reflect), "Z");
        assert_eq!(x.reflect_partial_eq(&Enum::X { value: 1 }), Some(true));
        assert_eq!(x.reflect_partial_eq(&Enum::Z), Some(false));
        assert_eq!(x.reflect_partial_cmp(&Enum::Y(0)), Some(Ordering::Less));
        assert_eq!(
            x.reflect_hash(),
            (&Enum::X { value: 1 } as &dyn Reflect).reflect_hash()
        );
        assert_ne!(
            x.reflect_hash(),
            (&Enum::X { value: 2 } as &dyn Reflect).reflect_hash()
        );
        let clone = (&Enum::Y(3) as &dyn Reflect).reflect_clone().unwrap();
        assert!(matches!(clone.downcast_ref::<Enum>(), Some(Enum::Y(3))));

        // Values of different types are never equal
        assert_eq!(reflect.reflect_partial_eq(&Unit), Some(false));
        assert_eq!(reflect.reflect_partial_cmp(&Unit), None);

        // Nested values
        let nested = vec![Some(Unnamed(1, 2.0)), None];
        assert_eq!(
            format!("{:?}", &nested as &dyn Reflect),
            "[Some(Unnamed(1, 2.0)), None]"
        );
        let clone = (&nested as &dyn Reflect).reflect_clone().unwrap();
        assert_eq!(clone.reflect_partial_eq(&nested), Some(true));
    }

    #[test]
    fn structural_hash() {
        #[derive(Reflect)]
        struct Pair {
            a: u32,
            b: u32,
        }

        #[derive(Reflect)]
        struct OtherPair {
            a: u32,
            b: u32,
        }

        #[derive(Reflect)]
        struct TuplePair(u32, u32);

        #[derive(Reflect)]
        struct Unit;

        #[derive(Reflect)]
        struct Nested {
            pairs: Vec<Pair>,
            unit: Unit,
        }

        #[derive(Reflect)]
        struct Float(f32);

        fn hash(value: &dyn Reflect) -> Option<u64> {
            value.reflect_hash()
        }

        // Equal values have equal hashes
        let pair = Pair { a: 1, b: 2 };
        assert!(hash(&pair).is_some());
        assert_eq!(hash(&pair), hash(&Pair { a: 1, b: 2 }));
        assert_eq!(hash(&TuplePair(1, 2)), hash(&TuplePair(1, 2)));
        assert_eq!(hash(&Unit), hash(&Unit));

        // Every field is hashed, in order
        assert_ne!(hash(&pair), hash(&Pair { a: 1, b: 3 }));
        assert_ne!(hash(&pair), hash(&Pair { a: 2, b: 1 }));
        assert_ne!(hash(&TuplePair(1, 2)), hash(&TuplePair(2, 1)));

        // Values of different types have different hashes
        assert_ne!(hash(&pair), hash(&OtherPair { a: 1, b: 2 }));
        assert_ne!(hash(&pair), hash(&TuplePair(1, 2)));

        let nested = Nested {
            pairs: vec![Pair { a: 1, b: 2 }],
            unit: Unit,
        };
        assert_eq!(
            hash(&nested),
            hash(&Nested {
                pairs: vec![Pair { a: 1, b: 2 }],
                unit: Unit,
            })
        );
        assert_ne!(
            hash(&nested),
            hash(&Nested {
                pairs: vec![Pair { a: 1, b: 3 }],
                unit: Unit,
            })
        );

        // A field that can't be hashed makes the struct unhashable
        assert!(hash(&Float(1.0)).is_none());
    }

    #[test]
    fn structural_hash_ignored_fields() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            cache: u32,
        }

        // Ignored fields are not compared, so they are not hashed either
        let foo = &Foo { a: 1, cache: 2 } as &dyn Reflect;
        let other = Foo { a: 1, cache: 3 };
        assert_eq!(foo.reflect_partial_eq(&other), Some(true));
        assert_eq!(foo.reflect_hash(), (&other as &dyn Reflect).reflect_hash());
        assert!(foo.reflect_hash().is_some());
    }

    #[test]
    fn structural_hash_custom_partial_eq() {
        // Equal values (according to the custom `PartialEq`) would have different structural hashes
        #[derive(Reflect)]
        #[reflect(PartialEq(|a, b| a.0 / 10 == b.0 / 10))]
        struct Struct(u32);

        #[derive(Reflect)]
        #[reflect(PartialEq(|a, b| a.value() / 10 == b.value() / 10))]
        enum Enum {
            A(u32),
        }

        impl Enum {
            fn value(&self) -> u32 {
                match self {
                    Enum::A(value) => *value,
                }
            }
        }

        let a = &Struct(11) as &dyn Reflect;
        assert_eq!(a.reflect_partial_eq(&Struct(19)), Some(true));
        assert!(a.reflect_hash().is_none());

        let a = &Enum::A(11) as &dyn Reflect;
        assert_eq!(a.reflect_partial_eq(&Enum::A(19)), Some(true));
        assert!(a.reflect_hash().is_none());

        // Containers of these values can't be hashed either
        assert!((&vec![Struct(11)] as &dyn Reflect).reflect_hash().is_none());

        // A registered `Hash` is used instead of the structural hash
        #[derive(Reflect)]
        #[reflect(
            PartialEq(|a, b| a.0 / 10 == b.0 / 10),
            Hash(|value| u64::from(value.0 / 10))
        )]
        struct WithHash(u32);

        let a = &WithHash(11) as &dyn Reflect;
        assert_eq!(a.reflect_partial_eq(&WithHash(19)), Some(true));
        assert!(a.reflect_hash().is_some());
        assert_eq!(
            a.reflect_hash(),
            (&WithHash(19) as &dyn Reflect).reflect_hash()
        );
    }

    #[test]
    fn builtin_attributes() {
        #[derive(Reflect, Debug, Clone, PartialEq, PartialOrd, Hash)]
        #[reflect(Debug, Clone, PartialEq, PartialOrd, Hash)]
        struct Foo {
            value: u32,
        }

        let foo = &Foo { value: 1 } as &dyn Reflect;
        assert!(Foo::TYPE.vtable().debug.is_some());
        assert_eq!(format!("{foo:?}"), "Foo { value: 1 }");
        assert_eq!(foo.reflect_partial_eq(&Foo { value: 1 }), Some(true));
        assert_eq!(
            foo.reflect_partial_cmp(&Foo { value: 2 }),
            Some(Ordering::Less)
        );
        // Unlike the structural implementation, the registered `Hash` supports structs
        assert_eq!(
            foo.reflect_hash(),
            (&Foo { value: 1 } as &dyn Reflect).reflect_hash()
        );
        let clone = foo.reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<Foo>(), Some(&Foo { value: 1 }));
    }

    #[test]
    fn builtin_attributes_custom() {
        #[derive(Reflect)]
        #[reflect(
            Debug(custom_debug),
            Clone(custom_clone),
            PartialEq(custom_eq),
            PartialOrd(custom_cmp),
            Hash(custom_hash)
        )]
        struct Foo(u32);

        fn custom_debug(value: &Foo, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "FooCustom<{}>", value.0)
        }
        fn custom_clone(value: &Foo) -> Foo {
            Foo(value.0 + 1)
        }
        fn custom_eq(a: &Foo, b: &Foo) -> bool {
            a.0 / 10 == b.0 / 10
        }
        fn custom_cmp(a: &Foo, b: &Foo) -> Option<Ordering> {
            b.0.partial_cmp(&a.0)
        }
        fn custom_hash(value: &Foo) -> u64 {
            u64::from(value.0 / 10)
        }

        let foo = &Foo(11) as &dyn Reflect;
        assert_eq!(format!("{foo:?}"), "FooCustom<11>");
        assert_eq!(
            foo.reflect_clone()
                .unwrap()
                .downcast_ref::<Foo>()
                .unwrap()
                .0,
            12
        );
        assert_eq!(foo.reflect_partial_eq(&Foo(19)), Some(true));
        assert_eq!(foo.reflect_partial_eq(&Foo(20)), Some(false));
        assert_eq!(foo.reflect_partial_cmp(&Foo(20)), Some(Ordering::Greater));
        assert_eq!(
            foo.reflect_hash(),
            (&Foo(15) as &dyn Reflect).reflect_hash()
        );
    }

    #[test]
    fn builtin_attributes_closures() {
        #[derive(Reflect)]
        #[reflect(
            Default(|| Foo(7)),
            Clone(|value| Foo(value.0 * 2)),
            Debug(|value, f| write!(f, "Foo<{}>", value.0)),
            PartialEq(|a, b| a.0 % 2 == b.0 % 2)
        )]
        struct Foo(u32);

        let foo = Foo::TYPE.default().unwrap();
        assert_eq!(format!("{foo:?}"), "Foo<7>");
        let clone = foo.reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<Foo>().unwrap().0, 14);
        assert_eq!(foo.reflect_partial_eq(&Foo(9)), Some(true));
        assert_eq!(foo.reflect_partial_eq(&Foo(8)), Some(false));
    }

    #[test]
    fn reflect_ignore_struct() {
        // Ignored fields don't need to be reflectable
        #[derive(Debug, Clone, PartialEq)]
        struct NotReflect(String);

        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            ignored: NotReflect,
            b: u32,
        }

        let Kind::Struct(struct_type) = Foo::TYPE.kind() else {
            panic!("expected struct kind");
        };
        assert!(struct_type.has_ignored_fields());
        let names = struct_type
            .fields()
            .iter()
            .map(|field| field.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["a", "b"]);

        let foo = Foo {
            a: 1,
            ignored: NotReflect(String::from("x")),
            b: 2,
        };
        let reflect = &foo as &dyn Reflect;
        assert_eq!(
            reflect
                .struct_ref()
                .unwrap()
                .field(1)
                .unwrap()
                .downcast_ref::<u32>(),
            Some(&2)
        );
        assert_eq!(format!("{reflect:?}"), "Foo { a: 1, b: 2, .. }");
        // Ignored fields are not compared
        let other = Foo {
            a: 1,
            ignored: NotReflect(String::from("y")),
            b: 2,
        };
        assert_eq!(reflect.reflect_partial_eq(&other), Some(true));
        assert_eq!(foo.ignored.0, "x");

        // Ignored fields can't be initialized or cloned structurally
        assert!(reflect.reflect_clone().is_none());
        // SAFETY: `func` is never called with an invalid pointer, because this returns `None`
        assert!(unsafe { Foo::TYPE.initialize_struct(|_, _| {}) }.is_none());

        #[derive(Reflect)]
        struct Tuple(#[reflect(ignore)] NotReflect, u32);
        let Kind::Struct(struct_type) = Tuple::TYPE.kind() else {
            panic!("expected struct kind");
        };
        // Fields keep their original names
        assert_eq!(struct_type.fields()[0].name, "1");
        let tuple = Tuple(NotReflect(String::from("x")), 3);
        assert_eq!(format!("{:?}", &tuple as &dyn Reflect), "Tuple(3, ..)");
        assert_eq!(tuple.0 .0, "x");
    }

    #[test]
    fn reflect_ignore_clone_attribute() {
        // A registered `Clone` still supports ignored fields
        #[derive(Reflect, Clone)]
        #[reflect(Clone, Default)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            ignored: String,
        }

        impl Default for Foo {
            fn default() -> Self {
                Self {
                    a: 1,
                    ignored: String::from("default"),
                }
            }
        }

        let foo = Foo::TYPE.default().unwrap();
        let clone = foo.reflect_clone().unwrap();
        let clone = clone.downcast_ref::<Foo>().unwrap();
        assert_eq!(clone.a, 1);
        assert_eq!(clone.ignored, "default");
    }

    #[test]
    fn reflect_ignore_enum() {
        struct NotReflect(u32);

        #[derive(Reflect)]
        enum Foo {
            A {
                #[reflect(ignore)]
                ignored: NotReflect,
                value: u32,
            },
            B(#[reflect(ignore)] NotReflect),
            C(u32),
        }

        let Kind::Enum(enum_type) = Foo::TYPE.kind() else {
            panic!("expected enum kind");
        };
        let variants = enum_type.variants();
        assert_eq!(variants[0].ignored_fields[0].name, "ignored");
        assert_eq!(variants[0].fields.len(), 1);
        assert_eq!(variants[0].fields[0].name, "value");
        assert_eq!(variants[1].ignored_fields.len(), 1);
        assert!(variants[1].fields.is_empty());
        assert!(variants[2].ignored_fields.is_empty());

        let mut a = Foo::A {
            ignored: NotReflect(1),
            value: 2,
        };
        let reflect = &mut a as &mut dyn Reflect;
        assert_eq!(format!("{reflect:?}"), "A { value: 2, .. }");
        // Field indices skip ignored fields
        *reflect
            .enum_mut()
            .unwrap()
            .field_mut(0)
            .unwrap()
            .downcast_mut::<u32>()
            .unwrap() = 20;
        assert!(reflect.enum_ref().unwrap().field(1).is_none());
        assert!(reflect.reflect_clone().is_none());
        let Foo::A { ignored, value } = &a else {
            panic!("expected `Foo::A`");
        };
        assert_eq!((ignored.0, *value), (1, 20));

        let b_value = Foo::B(NotReflect(3));
        let b = &b_value as &dyn Reflect;
        assert_eq!(format!("{b:?}"), "B(..)");
        assert!(b.reflect_clone().is_none());
        let Foo::B(ignored) = &b_value else {
            panic!("expected `Foo::B`");
        };
        assert_eq!(ignored.0, 3);
        // Variants without ignored fields can still be cloned
        let c = (&Foo::C(4) as &dyn Reflect).reflect_clone().unwrap();
        assert!(matches!(c.downcast_ref::<Foo>(), Some(Foo::C(4))));
    }

    #[test]
    fn reflect_ignore_default_and_clone() {
        #[derive(PartialEq, Debug)]
        struct NotDefault(u32);

        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore, default, clone)]
            cache: String,
            #[reflect(ignore, default(|| NotDefault(5)), clone(|value| NotDefault(value.0 + 1)))]
            custom: NotDefault,
        }

        let Kind::Struct(struct_type) = Foo::TYPE.kind() else {
            panic!("expected struct kind");
        };
        let ignored = struct_type
            .ignored_fields()
            .iter()
            .map(|field| field.name)
            .collect::<Vec<_>>();
        assert_eq!(ignored, ["cache", "custom"]);

        // SAFETY: the only reflected field is a `u32`, which is initialized
        let foo = unsafe {
            Foo::TYPE.initialize_struct(|field, ptr| {
                assert_eq!(field.name, "a");
                ptr.as_ptr().cast::<u32>().write(10);
            })
        }
        .unwrap();
        let foo = foo.downcast_ref::<Foo>().unwrap();
        assert_eq!(foo.a, 10);
        assert_eq!(foo.cache, "");
        assert_eq!(foo.custom, NotDefault(5));

        let foo = Foo {
            a: 1,
            cache: String::from("cache"),
            custom: NotDefault(1),
        };
        let clone = (&foo as &dyn Reflect).reflect_clone().unwrap();
        let clone = clone.downcast_ref::<Foo>().unwrap();
        assert_eq!(clone.a, 1);
        assert_eq!(clone.cache, "cache");
        assert_ne!(clone.cache.as_ptr(), foo.cache.as_ptr());
        assert_eq!(clone.custom, NotDefault(2));

        // `default` is required to initialize the struct, and `clone` is required to clone it
        #[derive(Reflect)]
        struct DefaultOnly {
            #[reflect(ignore, default)]
            ignored: String,
        }
        // SAFETY: there are no reflected fields
        assert!(unsafe { DefaultOnly::TYPE.initialize_struct(|_, _| {}) }.is_some());
        let value = DefaultOnly {
            ignored: String::new(),
        };
        assert!((&value as &dyn Reflect).reflect_clone().is_none());
        assert!(value.ignored.is_empty());

        #[derive(Reflect)]
        struct CloneOnly(#[reflect(ignore, clone)] String);
        // SAFETY: this returns `None`, so `func` is never called
        assert!(unsafe { CloneOnly::TYPE.initialize_struct(|_, _| {}) }.is_none());
        let value = CloneOnly(String::from("x"));
        let clone = (&value as &dyn Reflect).reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<CloneOnly>().unwrap().0, "x");
    }

    #[test]
    fn reflect_clone_failure_drops_cloned_fields() {
        use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

        static DROPS: AtomicUsize = AtomicUsize::new(0);

        #[derive(Reflect)]
        #[reflect(Clone(|value| Counted(value.0)))]
        struct Counted(u32);

        impl Drop for Counted {
            fn drop(&mut self) {
                DROPS.fetch_add(1, Relaxed);
            }
        }

        // Can't be cloned, because it has an ignored field without `clone`
        #[derive(Reflect)]
        struct NotCloneable(#[reflect(ignore)] u32);

        #[derive(Reflect)]
        struct Foo {
            a: Counted,
            b: Counted,
            c: NotCloneable,
            d: Counted,
        }

        #[derive(Reflect)]
        enum Bar {
            A(Counted, NotCloneable, Counted),
        }

        let foo = Foo {
            a: Counted(1),
            b: Counted(2),
            c: NotCloneable(3),
            d: Counted(4),
        };
        assert!((&foo as &dyn Reflect).reflect_clone().is_none());
        // The clones of `a` and `b` were dropped, and `d` was never cloned
        assert_eq!(DROPS.load(Relaxed), 2);
        drop(foo);
        assert_eq!(DROPS.load(Relaxed), 5);

        let bar = Bar::A(Counted(1), NotCloneable(2), Counted(3));
        assert!((&bar as &dyn Reflect).reflect_clone().is_none());
        assert_eq!(DROPS.load(Relaxed), 6);
        drop(bar);
        assert_eq!(DROPS.load(Relaxed), 8);
    }

    #[test]
    fn reflect_clone_enum_discriminant() {
        use core::marker::PhantomData;

        // Explicit discriminants are stored outside of the fields
        #[derive(Reflect, Debug, PartialEq)]
        #[repr(u8)]
        enum Explicit {
            A(u8) = 3,
            B { x: u8, y: String } = 200,
            C = 7,
            // Zero-sized fields can share an offset with other fields
            D(PhantomData<u8>, u16, PhantomData<u32>),
        }

        for value in [
            Explicit::A(1),
            Explicit::B {
                x: 2,
                y: String::from("y"),
            },
            Explicit::C,
            Explicit::D(PhantomData, 3, PhantomData),
        ] {
            let clone = (&value as &dyn Reflect).reflect_clone().unwrap();
            assert_eq!(clone.downcast_ref::<Explicit>(), Some(&value));
        }

        // The discriminant is stored in the niche of `bool` (so it overlaps the field of `A`)
        #[derive(Reflect, Debug, PartialEq)]
        enum Niche {
            A(bool),
            B,
            C,
        }
        assert_eq!(size_of::<Niche>(), 1);

        for value in [Niche::A(true), Niche::A(false), Niche::B, Niche::C] {
            let clone = (&value as &dyn Reflect).reflect_clone().unwrap();
            assert_eq!(clone.downcast_ref::<Niche>(), Some(&value));
        }
    }

    #[test]
    fn reflect_ignore_clone_enum() {
        #[derive(Reflect)]
        enum Foo {
            A {
                value: u32,
                #[reflect(ignore, clone)]
                name: String,
                #[reflect(ignore, clone(|value| value + 1))]
                count: u64,
            },
            B(u8, #[reflect(ignore, clone)] String),
        }

        let a = Foo::A {
            value: 1,
            name: String::from("name"),
            count: 2,
        };
        let reflect = &a as &dyn Reflect;
        let Foo::A { value, name, count } = &a else {
            panic!("expected `Foo::A`");
        };
        let enum_ref = reflect.enum_ref().unwrap();
        let base = core::ptr::from_ref(&a).addr();
        assert_eq!(
            enum_ref.field_offset(0),
            Some(core::ptr::from_ref(value).addr() - base)
        );
        // Reflected field indices don't include ignored fields
        assert_eq!(enum_ref.field_offset(1), None);
        assert!(enum_ref.field(1).is_none());
        assert_eq!(
            enum_ref.ignored_field_offset(0),
            Some(core::ptr::from_ref(name).addr() - base)
        );
        assert_eq!(
            enum_ref.ignored_field_offset(1),
            Some(core::ptr::from_ref(count).addr() - base)
        );
        assert_eq!(enum_ref.ignored_field_offset(2), None);

        let clone = reflect.reflect_clone().unwrap();
        let Some(Foo::A { value, name, count }) = clone.downcast_ref::<Foo>() else {
            panic!("expected `Foo::A`");
        };
        assert_eq!((*value, name.as_str(), *count), (1, "name", 3));

        let b = Foo::B(7, String::from("b"));
        let clone = (&b as &dyn Reflect).reflect_clone().unwrap();
        let Some(Foo::B(value, name)) = clone.downcast_ref::<Foo>() else {
            panic!("expected `Foo::B`");
        };
        assert_eq!((*value, name.as_str()), (7, "b"));
    }

    #[test]
    fn kind_ref() {
        #[derive(Reflect)]
        struct Foo {
            list: Vec<u32>,
        }

        let mut foo = Foo { list: vec![1] };
        let reflect = &mut foo as &mut dyn Reflect;
        let KindRef::Struct(struct_ref) = reflect.kind_ref() else {
            panic!("expected struct");
        };
        let KindRef::List(list) = struct_ref.field(0).unwrap().kind_ref() else {
            panic!("expected list");
        };
        assert_eq!(list.len(), 1);
        let crate::KindMut::Struct(mut struct_mut) = reflect.kind_mut() else {
            panic!("expected struct");
        };
        let mut list = struct_mut.field_mut(0).unwrap().list_mut().unwrap();
        list.push(Box::new(2u32)).unwrap();
        assert_eq!(foo.list, [1, 2]);
    }

    #[test]
    fn reflect_opaque() {
        // Fields of opaque types don't need to be reflectable
        struct NotReflect;

        #[derive(Reflect)]
        #[reflect(opaque, Debug)]
        #[derive(Debug)]
        struct Foo {
            #[expect(
                dead_code,
                reason = "only used to check that opaque fields aren't reflected"
            )]
            value: NotReflect,
        }

        impl fmt::Debug for NotReflect {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("NotReflect")
            }
        }

        assert!(matches!(
            Foo::TYPE.kind(),
            Kind::Primitive(crate::Primitive::Opaque)
        ));
        assert_eq!(Foo::TYPE.type_path().full(), "bevy_reflect2::tests::Foo");
        let foo = Foo { value: NotReflect };
        assert_eq!(
            format!("{:?}", &foo as &dyn Reflect),
            "Foo { value: NotReflect }"
        );

        #[derive(Reflect)]
        #[reflect(opaque)]
        struct Generic<T>(T);
        assert_eq!(
            Generic::<u32>::TYPE.type_path().full(),
            "bevy_reflect2::tests::Generic<u32>"
        );
    }

    #[test]
    fn reflect_where() {
        trait Assoc {
            type Value;
        }

        #[derive(Reflect)]
        struct Marker;

        impl Assoc for Marker {
            type Value = u32;
        }

        #[derive(Reflect)]
        #[reflect(where T::Value: crate::GetTypeRegistration)]
        struct Foo<T: Assoc> {
            value: T::Value,
        }

        let mut type_registry = TypeRegistry::new();
        type_registry.register::<Foo<Marker>>();
        let foo = Foo::<Marker> { value: 1 };
        let struct_ref = (&foo as &dyn Reflect).struct_ref().unwrap();
        assert_eq!(struct_ref.field(0).unwrap().downcast_ref::<u32>(), Some(&1));
    }

    #[test]
    fn reflect_ignored_generic_param() {
        use core::marker::PhantomData;

        // `F` is not `Typed`, so it is ignored
        #[derive(Reflect)]
        #[reflect(ignore_params(F))]
        struct Foo<T, F> {
            value: T,
            #[reflect(ignore)]
            _func: F,
        }

        fn func() {}
        fn make<F: Fn()>(func: F) -> Foo<u32, F> {
            Foo {
                value: 1,
                _func: func,
            }
        }
        let foo = make(func);
        let reflect = &foo as &dyn Reflect;
        assert_eq!(
            reflect
                .struct_ref()
                .unwrap()
                .field(0)
                .unwrap()
                .downcast_ref::<u32>(),
            Some(&1)
        );
        assert!(reflect
            .ty()
            .type_path()
            .full()
            .starts_with("bevy_reflect2::tests::Foo<u32, "));

        #[derive(Reflect, Clone)]
        #[reflect(ignore_params(T))]
        enum Bar<T> {
            A(#[reflect(ignore, clone)] PhantomData<T>),
        }
        let bar = Bar::<core::cell::Cell<u8>>::A(PhantomData);
        assert_eq!(
            (&bar as &dyn Reflect).ty().type_path().short(),
            "Bar<Cell<u8>>"
        );
    }

    #[test]
    fn reflect_enum_field_names_shadowing() {
        // Field names must not conflict with names used by the generated code
        #[derive(Reflect, Debug, PartialEq)]
        enum Foo {
            A { index: u32, variant: u64 },
        }

        let foo = Foo::A {
            index: 1,
            variant: 2,
        };
        let enum_ref = (&foo as &dyn Reflect).enum_ref().unwrap();
        assert_eq!(enum_ref.field(0).unwrap().downcast_ref::<u32>(), Some(&1));
        assert_eq!(enum_ref.field(1).unwrap().downcast_ref::<u64>(), Some(&2));
    }
}
