use crate::{
    FieldKind, GenericTypePath, GenericTypePathCell, GetTypeRegistration, Kind, Struct, StructKind,
    Type, TypePath, TypeRegistration, TypeVTable, Typed, UncoveredBytes,
};
use alloc::format;
use bevy_ptr::Ptr;
use bevy_utils::prelude::ShortName;
use core::{
    alloc::Layout,
    any::{type_name, TypeId},
    cmp::Ordering,
    hash::Hasher,
    marker::PhantomData,
};

// SAFETY: `PhantomData<T>` is a zero-sized unit struct without fields (zero-sized types have no
// bytes, so none of them need to be initialized), the layout is generated from `Self`,
// `cast_reflect` is generated for `Self`, and the shared vtable functions are valid for every
// `PhantomData<T>` (see above).
unsafe impl<T: ?Sized + 'static> Typed for PhantomData<T>
where
    PhantomData<T>: Send + Sync,
{
    const TYPE: &'static Type = &{
        // `clone` and `debug` use the structural implementations, which only rely on the
        // `cast_reflect` and type path of `Self`. They print the short type path, like the
        // `Debug` impl of `PhantomData<T>`.
        let vtable = TypeVTable {
            set_default: Some(phantom_set_default),
            hash: Some(phantom_hash),
            partial_eq: Some(phantom_partial_eq),
            partial_cmp: Some(phantom_partial_cmp),
            ..TypeVTable::new::<Self>()
        };
        // SAFETY: see the safety comment on this impl
        unsafe {
            Type::new(
                TypeId::of::<Self>(),
                Kind::Struct(Struct::new(
                    &[],
                    FieldKind::Unit,
                    StructKind::Struct,
                    &[],
                    UncoveredBytes::Padding,
                )),
                Some(Layout::new::<Self>()),
                vtable,
                TypePath::generic::<Self>("core::marker", "PhantomData"),
            )
        }
    };
}

impl<T: ?Sized + 'static> GetTypeRegistration for PhantomData<T>
where
    PhantomData<T>: Send + Sync,
{
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Self>()
    }
}

// `T` is usually not reflectable (ex: `PhantomData<fn() -> T>`), so it is not required to be `Typed`.
// Its path comes from `type_name` instead.
impl<T: ?Sized + 'static> GenericTypePath for PhantomData<T> {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            format!("core::marker::PhantomData<{}>", type_name::<T>())
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            format!("PhantomData<{}>", ShortName::of::<T>())
        })
    }
}

// Every `PhantomData<T>` is the same zero-sized value, and its trait impls ignore `T`, so these
// vtable functions are shared by all `PhantomData<T>` types instead of being generated for each `T`.
// This is sound because they never touch the (zero) bytes of the value, and never need to know its
// type. `cast_reflect` must still be generated for each `T`, since it determines the value's type
// (ex: `Reflect::ty` and `Any::type_id`).

/// Initializes a `PhantomData<T>`, which is zero-sized, so there is nothing to write.
fn phantom_set_default(_ptr: *mut u8) {}

/// Hashes a `PhantomData<T>`, which (like its `Hash` impl) writes nothing.
fn phantom_hash(_ptr: Ptr<'_>, _hasher: &mut dyn Hasher) {}

/// Compares two `PhantomData<T>` values, which are always equal.
fn phantom_partial_eq(_a: Ptr<'_>, _b: Ptr<'_>) -> bool {
    true
}

/// Compares two `PhantomData<T>` values, which are always equal.
fn phantom_partial_cmp(_a: Ptr<'_>, _b: Ptr<'_>) -> Option<Ordering> {
    Some(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use crate as bevy_reflect2;
    use crate::{Reflect, TypeRegistry};
    use alloc::{boxed::Box, format};
    use core::{any::TypeId, marker::PhantomData};

    struct NotReflect;

    #[test]
    fn phantom_data() {
        let value = PhantomData::<fn() -> NotReflect>;
        let reflect = &value as &dyn Reflect;
        assert_eq!(
            reflect.ty().type_path().full(),
            "core::marker::PhantomData<fn() -> bevy_reflect2::impls::core::marker::tests::NotReflect>"
        );
        assert_eq!(
            reflect.ty().type_path().short(),
            "PhantomData<fn() -> NotReflect>"
        );
        assert_eq!(reflect.ty().type_path().type_ident(), "PhantomData");
        assert_eq!(reflect.ty().type_path().crate_name(), Some("core"));
        assert_eq!(reflect.ty().type_path().module_path(), Some("core::marker"));

        let clone: Box<dyn Reflect> = reflect.reflect_clone().unwrap();
        assert!(clone.is::<PhantomData<fn() -> NotReflect>>());
        assert_eq!(reflect.reflect_partial_eq(&*clone), Some(true));
        assert!(reflect.reflect_hash().is_some());
        assert_eq!(
            reflect.reflect_partial_cmp(&*clone),
            Some(core::cmp::Ordering::Equal)
        );
        assert_eq!(format!("{reflect:?}"), "PhantomData<fn() -> NotReflect>");
        // Shared vtable functions must not erase the type of each `PhantomData<T>`
        assert!(!(&PhantomData::<u32> as &dyn Reflect).is::<PhantomData<fn() -> NotReflect>>());
        assert_ne!(
            (&PhantomData::<u32> as &dyn Reflect).ty().type_id(),
            reflect.ty().type_id()
        );
        assert!(reflect
            .ty()
            .default()
            .unwrap()
            .is::<PhantomData<fn() -> NotReflect>>());

        let mut registry = TypeRegistry::empty();
        registry.register::<PhantomData<u32>>();
        assert!(registry.contains(TypeId::of::<PhantomData<u32>>()));
    }

    #[test]
    fn phantom_data_field() {
        #[derive(Reflect)]
        struct Marked {
            value: u32,
            marker: PhantomData<fn() -> NotReflect>,
        }

        let value = Marked {
            value: 3,
            marker: PhantomData,
        };
        let clone = (&value as &dyn Reflect).reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<Marked>().unwrap().value, 3);
    }
}
