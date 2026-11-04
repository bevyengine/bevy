//! Reflection for tuples with 1 to 12 elements. The unit tuple `()` is a [`Primitive`](crate::kind::Primitive).
//!
//! Tuples are described as structs with [`StructKind::Tuple`] and unnamed fields, named by their
//! index (ex: `0`). Their element types are not known to implement any traits, so [`Clone`],
//! [`Debug`](core::fmt::Debug), [`Hash`](core::hash::Hash), [`PartialEq`], and [`PartialOrd`] use
//! the structural implementations (which fail if an element doesn't support the operation), and
//! they cannot be defaulted through reflection.

use crate::{
    attributes::CustomAttributes, kind::FieldKind, kind::Kind, kind::Struct, kind::StructField,
    kind::StructKind, kind::UncoveredBytes, type_path::GenericTypePath,
    type_path::GenericTypePathCell, type_path::TypePath, type_path::TypePathStr,
    type_registry::GetTypeRegistration, type_registry::TypeRegistration,
    type_registry::TypeRegistry, type_vtable::TypeVTable, Type, TypeRef, Typed,
};
use alloc::string::String;
use core::{alloc::Layout, any::TypeId, mem::offset_of};

/// Formats the path of a tuple with the given element paths (ex: `(u32, f32)` or `(u32,)`).
fn tuple_path<'a>(elements: impl ExactSizeIterator<Item = &'a str>) -> String {
    let len = elements.len();
    let mut path = String::from("(");
    for (index, element) in elements.enumerate() {
        if index > 0 {
            path.push_str(", ");
        }
        path.push_str(element);
    }
    if len == 1 {
        path.push(',');
    }
    path.push(')');
    path
}

macro_rules! impl_reflect_tuple {
    ($($index:tt: $T:ident),+) => {
        impl<$($T: Typed),+> GenericTypePath for ($($T,)+) {
            fn type_path() -> &'static str {
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert(TypeId::of::<Self>(), || {
                    tuple_path([$($T::TYPE.type_path().full()),+].into_iter())
                })
            }

            fn short_type_path() -> &'static str {
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert(TypeId::of::<Self>(), || {
                    tuple_path([$($T::TYPE.type_path().short()),+].into_iter())
                })
            }
        }

        // SAFETY: `fields` contains every element of the tuple, with offsets and types generated
        // from `Self`, so every other byte is padding. The layout and vtable are generated for
        // `Self`.
        unsafe impl<$($T: Typed),+> Typed for ($($T,)+) {
            const TYPE: &'static Type = &{
                let fields = &[$(
                    StructField {
                        name: stringify!($index),
                        offset: offset_of!(Self, $index),
                        ty: TypeRef(Type::of::<$T>),
                        custom_attributes: CustomAttributes::EMPTY,
                    }
                ),+];
                let vtable = TypeVTable::new::<Self>();
                // SAFETY: see the safety comment on this impl
                unsafe {
                    Type::new(
                        TypeId::of::<Self>(),
                        Kind::Struct(Struct::new(
                            fields,
                            FieldKind::Unnamed,
                            StructKind::Tuple,
                            &[],
                            UncoveredBytes::Padding,
                        )),
                        Some(Layout::new::<Self>()),
                        vtable,
                        // Tuples have no name, crate, or module
                        TypePath::new(
                            TypePathStr::Lazy(<Self as GenericTypePath>::type_path),
                            TypePathStr::Lazy(<Self as GenericTypePath>::short_type_path),
                            0..0,
                            None,
                            None,
                        ),
                    )
                }
            };
        }

        impl<$($T: Typed + GetTypeRegistration),+> GetTypeRegistration for ($($T,)+) {
            fn get_type_registration() -> TypeRegistration {
                TypeRegistration::of::<Self>()
            }

            fn register_type_dependencies(registry: &mut TypeRegistry) {
                $(registry.register::<$T>();)+
            }
        }
    };
}

impl_reflect_tuple!(0: A);
impl_reflect_tuple!(0: A, 1: B);
impl_reflect_tuple!(0: A, 1: B, 2: C);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K);
impl_reflect_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L);

#[cfg(test)]
mod tests {
    use crate as bevy_reflect2;
    use crate::{
        kind::FieldKind, kind::Kind, kind::StructKind, type_registry::TypeRegistry, Reflect, Type,
    };
    use alloc::{format, string::String, vec, vec::Vec};
    use core::{any::TypeId, cmp::Ordering};

    #[test]
    fn tuple_type() {
        let ty = Type::of::<(u8, String, u64)>();
        let Kind::Struct(struct_type) = ty.kind() else {
            panic!("tuples should be structs");
        };
        assert_eq!(*struct_type.field_kind(), FieldKind::Unnamed);
        assert_eq!(struct_type.struct_kind(), StructKind::Tuple);
        let fields: Vec<_> = struct_type
            .fields()
            .iter()
            .map(|field| (field.name, field.ty.get().type_id()))
            .collect();
        assert_eq!(
            fields,
            [
                ("0", TypeId::of::<u8>()),
                ("1", TypeId::of::<String>()),
                ("2", TypeId::of::<u64>()),
            ]
        );
        assert_eq!(ty.type_path().full(), "(u8, alloc::string::String, u64)");
        assert_eq!(ty.type_path().short(), "(u8, String, u64)");
        assert_eq!(ty.type_path().crate_name(), None);
        assert_eq!(ty.type_path().module_path(), None);
        assert_eq!(Type::of::<(u32,)>().type_path().full(), "(u32,)");
    }

    #[test]
    fn tuple_access() {
        let mut value = (1u8, String::from("a"), 3.0f32);
        let reflect = &mut value as &mut dyn Reflect;
        let fields = reflect.struct_ref().unwrap();
        assert_eq!(fields.field(0).unwrap().downcast_ref::<u8>(), Some(&1));
        assert_eq!(
            fields.field(1).unwrap().downcast_ref::<String>().unwrap(),
            "a"
        );
        assert_eq!(fields.field(2).unwrap().downcast_ref::<f32>(), Some(&3.0));

        *reflect
            .struct_mut()
            .unwrap()
            .field_mut(1)
            .unwrap()
            .downcast_mut::<String>()
            .unwrap() = String::from("b");
        assert_eq!(value.1, "b");
    }

    #[test]
    fn tuple_traits() {
        let value = (1u8, String::from("a"));
        let reflect = &value as &dyn Reflect;

        let clone = reflect.reflect_clone().unwrap();
        assert_eq!(
            clone.downcast_ref::<(u8, String)>(),
            Some(&(1, String::from("a")))
        );
        assert_eq!(reflect.reflect_partial_eq(&*clone).ok(), Some(true));
        assert_eq!(
            reflect.reflect_partial_eq(&(2u8, String::from("a"))).ok(),
            Some(false)
        );
        assert_eq!(
            reflect
                .reflect_partial_cmp(&(2u8, String::from("a")))
                .ok()
                .flatten(),
            Some(Ordering::Less)
        );
        // Hashed structurally, using the hash of each element
        assert_eq!(
            reflect.reflect_hash().ok(),
            (&(1u8, String::from("a")) as &dyn Reflect)
                .reflect_hash()
                .ok()
        );
        assert_ne!(
            reflect.reflect_hash().ok(),
            (&(2u8, String::from("a")) as &dyn Reflect)
                .reflect_hash()
                .ok()
        );
        assert!((&(1u8, 1.0f32) as &dyn Reflect).reflect_hash().is_err());
        assert!(reflect.ty().default().is_none());

        assert_eq!(format!("{reflect:?}"), format!("{value:?}"));
        assert_eq!(format!("{:?}", &(1u8,) as &dyn Reflect), "(1,)");
    }

    #[test]
    fn tuple_fields() {
        #[derive(Reflect)]
        struct Foo {
            pairs: Vec<(u32, f32)>,
            nested: ((u8, u16), bool),
        }

        let value = Foo {
            pairs: vec![(1, 2.0)],
            nested: ((3, 4), true),
        };
        let clone = (&value as &dyn Reflect).reflect_clone().unwrap();
        let clone = clone.downcast_ref::<Foo>().unwrap();
        assert_eq!(clone.pairs, value.pairs);
        assert_eq!(clone.nested, value.nested);
    }

    #[test]
    fn register_tuple() {
        let mut registry = TypeRegistry::empty();
        registry.register::<(u32, (u8, bool))>();
        assert!(registry.contains(TypeId::of::<(u32, (u8, bool))>()));
        assert!(registry.contains(TypeId::of::<(u8, bool)>()));
        assert!(registry.contains(TypeId::of::<u32>()));
        assert!(registry.get_with_type_path("(u32, (u8, bool))").is_some());
    }
}
