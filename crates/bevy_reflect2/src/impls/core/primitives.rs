use crate::{
    self as bevy_reflect2, impl_reflect_primitive, GetTypeRegistration, Kind, Type, TypePath,
    TypePathStr, TypeRegistration, TypeVTable, Typed,
};
use core::{alloc::Layout, any::TypeId};

impl_reflect_primitive!(
    #[primitive(Bool)]
    bool(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Char)]
    char(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    u8(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    u16(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    u32(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    u64(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    u128(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    usize(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    i8(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    i16(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    i32(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    i64(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    i128(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Int)]
    isize(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Float)]
    f32(Clone, Debug, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Float)]
    f64(Clone, Debug, PartialEq, PartialOrd, Default)
);
impl_reflect_primitive!(
    #[primitive(Unit)]
    #[type_path = "()"]
    ()(Clone, Debug, Hash, PartialEq, PartialOrd, Default)
);

impl_reflect_primitive!(
    #[type_path = "&str"]
    &'static str(
        Clone,
        Debug,
        Hash,
        PartialEq,
        PartialOrd,
        Default
    )
);

// SAFETY: every item of `[T; N]` is a `T`, laid out contiguously with a stride of `size_of::<T>()`
unsafe impl<T: Typed, const N: usize> Typed for [T; N] {
    // SAFETY: `[T; N]` stores `N` contiguous `T` values, each `size_of::<T>()` bytes apart
    const TYPE: &'static Type = &unsafe {
        Type::new(
            TypeId::of::<Self>(),
            Kind::Array(crate::Array::new(
                crate::TypeRef(Type::of::<T>),
                N,
                size_of::<T>(),
            )),
            Some(Layout::new::<Self>()),
            TypeVTable::new::<Self>(),
            // Arrays have no type identifier, crate, or module
            TypePath::new(
                TypePathStr::Lazy(<Self as crate::GenericTypePath>::type_path),
                TypePathStr::Lazy(<Self as crate::GenericTypePath>::short_type_path),
                0..0,
                None,
                None,
            ),
        )
    };
}

impl<T: Typed, const N: usize> crate::GenericTypePath for [T; N] {
    fn type_path() -> &'static str {
        static CELL: crate::GenericTypePathCell = crate::GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            alloc::format!("[{}; {N}]", T::TYPE.type_path().full())
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: crate::GenericTypePathCell = crate::GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            alloc::format!("[{}; {N}]", T::TYPE.type_path().short())
        })
    }
}

impl<T: Typed + GetTypeRegistration, const N: usize> GetTypeRegistration for [T; N] {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Self>()
    }

    fn register_type_dependencies(registry: &mut crate::TypeRegistry) {
        registry.register::<T>();
    }
}

#[cfg(test)]
mod tests {
    use crate::{Reflect, Typed};
    use alloc::string::ToString;

    #[test]
    fn primitive_builtins() {
        let a = &10u32 as &dyn Reflect;
        let b = &20u32 as &dyn Reflect;
        assert_eq!(a.reflect_partial_eq(&10u32), Some(true));
        assert_eq!(a.reflect_partial_eq(b), Some(false));
        assert_eq!(a.reflect_partial_eq(&10u64), Some(false));
        assert_eq!(a.reflect_partial_cmp(b), Some(core::cmp::Ordering::Less));
        assert_eq!(a.reflect_hash(), (&10u32 as &dyn Reflect).reflect_hash());
        assert_ne!(a.reflect_hash(), b.reflect_hash());
        assert_eq!(alloc::format!("{a:?}"), "10");
        assert_eq!(
            *a.reflect_clone().unwrap().downcast_ref::<u32>().unwrap(),
            10
        );
        assert!((&1.0f32 as &dyn Reflect).reflect_hash().is_none());
    }

    #[test]
    fn static_str() {
        let value: &'static str = "hello";
        let reflect = &value as &dyn Reflect;
        assert_eq!(reflect.ty().type_path().full(), "&str");
        assert_eq!(reflect.ty().type_path().short(), "&str");
        assert_eq!(reflect.ty().type_path().type_ident(), "str");
        assert_eq!(reflect.ty().type_path().crate_name(), None);
        assert_eq!(reflect.reflect_partial_eq(&"hello"), Some(true));
        assert_eq!(alloc::format!("{reflect:?}"), "\"hello\"");
        let default = <&'static str>::TYPE.default().unwrap();
        assert_eq!(*default.downcast_ref::<&'static str>().unwrap(), "");
    }

    #[test]
    fn array() {
        let mut value = [1u32, 2, 3];
        assert_eq!(<[u32; 3]>::TYPE.type_path().full(), "[u32; 3]");
        let reflect = &mut value as &mut dyn Reflect;
        {
            let array = reflect.array_ref().unwrap();
            assert_eq!(array.len(), 3);
            assert_eq!(array.get(1).unwrap().downcast_ref::<u32>(), Some(&2));
            assert!(array.get(3).is_none());
            let items = array
                .iter()
                .map(|item| *item.downcast_ref::<u32>().unwrap())
                .collect::<alloc::vec::Vec<_>>();
            assert_eq!(items, [1, 2, 3]);
        }
        *reflect
            .array_mut()
            .unwrap()
            .get_mut(2)
            .unwrap()
            .downcast_mut::<u32>()
            .unwrap() = 10;
        assert_eq!(alloc::format!("{reflect:?}"), "[1, 2, 10]");
        assert_eq!(reflect.reflect_partial_eq(&[1u32, 2, 10]), Some(true));
        assert_eq!(reflect.reflect_partial_eq(&[1u32, 2, 11]), Some(false));
        assert_eq!(
            reflect.reflect_partial_cmp(&[1u32, 3, 0]),
            Some(core::cmp::Ordering::Less)
        );
        assert!(reflect.reflect_hash().is_some());
        let clone = reflect.reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<[u32; 3]>(), Some(&[1, 2, 10]));
        assert_eq!(value, [1, 2, 10]);

        // Zero-sized items
        let units = [(), ()];
        let array = (&units as &dyn Reflect).array_ref().unwrap();
        assert_eq!(array.iter().count(), 2);
        assert_eq!(
            "[(), ()]",
            alloc::format!("{:?}", &units as &dyn Reflect).to_string()
        );
    }
}
