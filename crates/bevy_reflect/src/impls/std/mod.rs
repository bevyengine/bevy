mod any;
mod borrow;
mod collections;
#[cfg(feature = "std")]
mod ffi;
mod hash;
mod net;
mod num;
mod ops;
mod option;
mod panic;
#[cfg(feature = "std")]
mod path;
mod primitives;
mod result;
mod string;
mod sync;
mod time;

#[cfg(test)]
mod tests {
    use crate::{
        Enum, FromReflect, PartialReflect, Reflect, ReflectSerialize, TypeInfo, TypeRegistry,
        Typed, VariantInfo, VariantType,
    };
    use alloc::{collections::BTreeMap, string::String, vec};
    use bevy_platform::collections::HashMap;
    use bevy_platform::time::Instant;
    use core::{
        f32::consts::{PI, TAU},
        time::Duration,
    };
    use static_assertions::assert_impl_all;
    use std::path::Path;

    #[test]
    fn can_serialize_duration() {
        let mut type_registry = TypeRegistry::default();
        type_registry.register::<Duration>();

        let reflect_serialize = type_registry
            .get_type_data::<ReflectSerialize>(core::any::TypeId::of::<Duration>())
            .unwrap();
        let _serializable = reflect_serialize.get_serializable(&Duration::ZERO);
    }

    #[test]
    fn should_partial_eq_char() {
        let a: &dyn PartialReflect = &'x';
        let b: &dyn PartialReflect = &'x';
        let c: &dyn PartialReflect = &'o';
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_i32() {
        let a: &dyn PartialReflect = &123_i32;
        let b: &dyn PartialReflect = &123_i32;
        let c: &dyn PartialReflect = &321_i32;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_f32() {
        let a: &dyn PartialReflect = &PI;
        let b: &dyn PartialReflect = &PI;
        let c: &dyn PartialReflect = &TAU;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_string() {
        let a: &dyn PartialReflect = &String::from("Hello");
        let b: &dyn PartialReflect = &String::from("Hello");
        let c: &dyn PartialReflect = &String::from("World");
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_vec() {
        let a: &dyn PartialReflect = &vec![1, 2, 3];
        let b: &dyn PartialReflect = &vec![1, 2, 3];
        let c: &dyn PartialReflect = &vec![3, 2, 1];
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_hash_map() {
        let mut a = <HashMap<_, _>>::default();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = <HashMap<_, _>>::default();
        c.insert(0usize, 3.21_f64);

        let a: &dyn PartialReflect = &a;
        let b: &dyn PartialReflect = &b;
        let c: &dyn PartialReflect = &c;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_btree_map() {
        let mut a = BTreeMap::new();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = BTreeMap::new();
        c.insert(0usize, 3.21_f64);

        let a: &dyn Reflect = &a;
        let b: &dyn Reflect = &b;
        let c: &dyn Reflect = &c;
        assert!(a
            .reflect_partial_eq(b.as_partial_reflect())
            .unwrap_or_default());
        assert!(!a
            .reflect_partial_eq(c.as_partial_reflect())
            .unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_option() {
        let a: &dyn PartialReflect = &Some(123);
        let b: &dyn PartialReflect = &Some(123);
        assert_eq!(Some(true), a.reflect_partial_eq(b));
    }

    #[test]
    fn option_should_impl_enum() {
        assert_impl_all!(Option<()>: Enum);

        let mut value = Some(123usize);

        assert!(value
            .reflect_partial_eq(&Some(123usize))
            .unwrap_or_default());
        assert!(!value
            .reflect_partial_eq(&Some(321usize))
            .unwrap_or_default());

        assert_eq!("Some", value.variant_name());
        assert_eq!("core::option::Option<usize>::Some", value.variant_path());

        if value.is_variant(VariantType::Tuple) {
            if let Some(field) = value
                .field_at_mut(0)
                .and_then(|field| field.try_downcast_mut::<usize>())
            {
                *field = 321;
            }
        } else {
            panic!("expected `VariantType::Tuple`");
        }

        assert_eq!(Some(321), value);
    }

    #[test]
    fn option_should_from_reflect() {
        #[derive(Reflect, PartialEq, Debug)]
        struct Foo(usize);

        let expected = Some(Foo(123));
        let output = <Option<Foo> as FromReflect>::from_reflect(&expected).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn option_should_apply() {
        #[derive(Reflect, PartialEq, Debug)]
        struct Foo(usize);

        // === None on None === //
        let patch = None::<Foo>;
        let mut value = None::<Foo>;
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto None");

        // === Some on None === //
        let patch = Some(Foo(123));
        let mut value = None::<Foo>;
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto None");

        // === None on Some === //
        let patch = None::<Foo>;
        let mut value = Some(Foo(321));
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto Some");

        // === Some on Some === //
        let patch = Some(Foo(123));
        let mut value = Some(Foo(321));
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto Some");
    }

    #[test]
    fn option_should_impl_typed() {
        assert_impl_all!(Option<()>: Typed);

        type MyOption = Option<i32>;
        let info = MyOption::type_info();
        if let TypeInfo::Enum(info) = info {
            assert_eq!(
                "None",
                info.variant_at(0).unwrap().name(),
                "Expected `None` to be variant at index `0`"
            );
            assert_eq!(
                "Some",
                info.variant_at(1).unwrap().name(),
                "Expected `Some` to be variant at index `1`"
            );
            assert_eq!("Some", info.variant("Some").unwrap().name());
            if let VariantInfo::Tuple(variant) = info.variant("Some").unwrap() {
                assert!(
                    variant.field_at(0).unwrap().is::<i32>(),
                    "Expected `Some` variant to contain `i32`"
                );
                assert!(
                    variant.field_at(1).is_none(),
                    "Expected `Some` variant to only contain 1 field"
                );
            } else {
                panic!("Expected `VariantInfo::Tuple`");
            }
        } else {
            panic!("Expected `TypeInfo::Enum`");
        }
    }

    #[test]
    fn nonzero_usize_impl_reflect_from_reflect() {
        let a: &dyn PartialReflect = &core::num::NonZero::<usize>::new(42).unwrap();
        let b: &dyn PartialReflect = &core::num::NonZero::<usize>::new(42).unwrap();
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        let forty_two: core::num::NonZero<usize> = FromReflect::from_reflect(a).unwrap();
        assert_eq!(forty_two, core::num::NonZero::<usize>::new(42).unwrap());
    }

    #[test]
    fn instant_should_from_reflect() {
        let expected = Instant::now();
        let output = <Instant as FromReflect>::from_reflect(&expected).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn path_should_from_reflect() {
        let path = Path::new("hello_world.rs");
        let output = <&'static Path as FromReflect>::from_reflect(&path).unwrap();
        assert_eq!(path, output);
    }

    #[test]
    fn type_id_should_from_reflect() {
        let type_id = core::any::TypeId::of::<usize>();
        let output = <core::any::TypeId as FromReflect>::from_reflect(&type_id).unwrap();
        assert_eq!(type_id, output);
    }

    #[test]
    fn static_str_should_from_reflect() {
        let expected = "Hello, World!";
        let output = <&'static str as FromReflect>::from_reflect(&expected).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    #[expect(
        unused_qualifications,
        reason = "ensures that we are testing the correct `HashMap` exports"
    )]
    fn should_reflect_hashmaps() {
        assert_impl_all!(std::collections::HashMap<u32, f32>: Reflect);
        assert_impl_all!(bevy_platform::collections::HashMap<u32, f32>: Reflect);

        // We specify `foldhash::fast::RandomState` directly here since without the `default-hasher`
        // feature, hashbrown uses an empty enum to force users to specify their own
        #[cfg(feature = "hashbrown")]
        assert_impl_all!(hashbrown::HashMap<u32, f32, foldhash::fast::RandomState>: Reflect);
    }
}
