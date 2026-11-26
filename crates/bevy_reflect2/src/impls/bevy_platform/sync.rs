use crate::{
    kind::Kind, kind::Pointer, kind::PointerVTable, kind::ReflectPointee,
    type_path::GenericTypePath, type_path::GenericTypePathCell, type_path::TypePath,
    type_registry::GetTypeRegistration, type_registry::TypeRegistration,
    type_registry::TypeRegistry, type_vtable, type_vtable::TypeVTable, Reflect, Type, TypeRef,
    Typed,
};
use alloc::boxed::Box;
use bevy_platform::sync::Arc;
use bevy_ptr::PtrMut;
use core::{alloc::Layout, any::TypeId, ptr};

// SAFETY: `Arc<T>` is described by `arc_vtable::<T>`, which is valid for `Arc<T>`.
// `ReflectPointee::TYPE` is either `None` or the type of `T`, the value the `Arc` points to. The
// layout and vtable are derived from `Self`.
unsafe impl<T: ?Sized + ReflectPointee> Typed for Arc<T> {
    // SAFETY: see above
    const TYPE: &'static Type = &unsafe {
        Type::new(
            TypeId::of::<Self>(),
            Kind::Pointer(Pointer::new(
                TypeRef(Type::of::<Self>),
                T::TYPE,
                arc_vtable::<T>(),
            )),
            Some(Layout::new::<Self>()),
            // Cloning an `Arc` shares its value (like its `Clone` impl) rather than cloning the
            // value structurally. This also makes `Arc`s of trait objects cloneable.
            TypeVTable {
                clone: Some(type_vtable::clone::<Self>),
                ..TypeVTable::new::<Self>()
            },
            TypePath::generic::<Self>("bevy_platform::sync", "Arc"),
        )
    };
}

/// Returns the [`PointerVTable`] of `Arc<T>`.
///
/// Unlike `Box<T>`, the layout of the allocation an `Arc<T>` points to is unspecified (it stores
/// reference counts next to the value), so these functions are generated for every `T`.
const fn arc_vtable<T: ?Sized + ReflectPointee>() -> PointerVTable {
    PointerVTable {
        cast_pointee: arc_cast_pointee::<T>,
        cast_pointee_mut: arc_cast_pointee_mut::<T>,
        set: arc_set::<T>,
        wrap: arc_wrap::<T>,
    }
}

/// # Safety
///
/// `ptr` must be a valid `Arc<T>`.
unsafe fn arc_cast_pointee<T: ?Sized + ReflectPointee>(
    _pointer: &'static Pointer,
    ptr: *const u8,
) -> *const dyn Reflect {
    // SAFETY: the caller guarantees that `ptr` is a valid `Arc<T>`
    let arc = unsafe { &*ptr.cast::<Arc<T>>() };
    T::cast_reflect(&**arc)
}

/// # Safety
///
/// `ptr` must be a valid `Arc<T>`.
unsafe fn arc_cast_pointee_mut<T: ?Sized + ReflectPointee>(
    _pointer: &'static Pointer,
    ptr: PtrMut<'_>,
) -> Option<*mut dyn Reflect> {
    // SAFETY: the caller guarantees that `ptr` is a valid `Arc<T>`
    let arc = unsafe { ptr.deref_mut::<Arc<T>>() };
    // `Arc::get_mut` only succeeds if this is the only `Arc` (or `Weak`) of the value, in which
    // case the value can be written to for as long as this `Arc` is exclusively borrowed
    let value = Arc::get_mut(arc)?;
    Some(T::cast_reflect(ptr::from_mut(value).cast_const()).cast_mut())
}

/// # Safety
///
/// `ptr` must be a valid `Arc<T>`.
unsafe fn arc_set<T: ?Sized + ReflectPointee>(
    _pointer: &'static Pointer,
    ptr: PtrMut<'_>,
    value: Box<dyn Reflect>,
) -> Result<(), Box<dyn Reflect>> {
    // SAFETY: the caller guarantees that `ptr` is a valid `Arc<T>`
    *unsafe { ptr.deref_mut::<Arc<T>>() } = Arc::from(T::from_reflect_box(value)?);
    Ok(())
}

fn arc_wrap<T: ?Sized + ReflectPointee>(
    _pointer: &'static Pointer,
    value: Box<dyn Reflect>,
) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
    Ok(Box::new(Arc::<T>::from(T::from_reflect_box(value)?)))
}

impl<T: ?Sized + ReflectPointee> GenericTypePath for Arc<T> {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            alloc::format!("bevy_platform::sync::Arc<{}>", T::type_path())
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            alloc::format!("Arc<{}>", T::short_type_path())
        })
    }
}

impl<T: ?Sized + ReflectPointee> GetTypeRegistration for Arc<T> {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Self>()
    }

    fn register_type_dependencies(registry: &mut TypeRegistry) {
        T::register_type_dependencies(registry);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_reflect2, impl_reflect_pointee, kind::Kind, type_registry::TypeRegistry,
        Reflect, Typed,
    };
    use alloc::{boxed::Box, format, string::String};
    use bevy_platform::sync::Arc;
    use core::any::TypeId;

    #[test]
    fn arc_sized() {
        assert_eq!(
            Arc::<u32>::TYPE.type_path().full(),
            "bevy_platform::sync::Arc<u32>"
        );
        assert_eq!(Arc::<u32>::TYPE.type_path().short(), "Arc<u32>");
        assert_eq!(
            Arc::<u32>::TYPE.type_path().crate_name(),
            Some("bevy_platform")
        );
        let Kind::Pointer(pointer_type) = Arc::<u32>::TYPE.kind() else {
            panic!("Expected pointer kind");
        };
        assert_eq!(
            pointer_type.pointee().unwrap().get().type_id(),
            TypeId::of::<u32>()
        );

        let mut value = Arc::new(1u32);
        let reflect = &mut value as &mut dyn Reflect;
        assert_eq!(
            reflect.pointer_ref().unwrap().get().downcast_ref(),
            Some(&1u32)
        );
        // The value is unique, so it can be mutated
        *reflect
            .pointer_mut()
            .unwrap()
            .get_mut()
            .unwrap()
            .downcast_mut::<u32>()
            .unwrap() = 2;
        reflect.pointer_mut().unwrap().set(Box::new(3u32)).unwrap();
        let rejected = reflect
            .pointer_mut()
            .unwrap()
            .set(Box::new(4u64))
            .unwrap_err();
        assert_eq!(rejected.downcast_ref(), Some(&4u64));
        assert_eq!(format!("{reflect:?}"), "3");

        assert_eq!(
            reflect.reflect_partial_cmp(&Arc::new(4u32)).unwrap(),
            Some(core::cmp::Ordering::Less)
        );
        assert_eq!(
            reflect.reflect_hash().unwrap(),
            (&Arc::new(3u32) as &dyn Reflect).reflect_hash().unwrap()
        );
        assert_eq!(*value, 3);
    }

    #[test]
    fn arc_shared() {
        let mut value = Arc::new(String::from("a"));
        let reflect = &mut value as &mut dyn Reflect;

        // Clones share the value, like `Arc::clone`
        let clone = reflect.reflect_clone().unwrap();
        let clone = clone.downcast_ref::<Arc<String>>().unwrap();
        assert!(Arc::ptr_eq(reflect.downcast_ref().unwrap(), clone));
        assert_eq!(reflect.reflect_partial_eq(clone).ok(), Some(true));

        // Shared values can't be mutated
        assert!(reflect.pointer_mut().unwrap().get_mut().is_none());

        // Setting the value points this `Arc` to a new value, leaving the shared value unchanged
        reflect
            .pointer_mut()
            .unwrap()
            .set(Box::new(String::from("b")))
            .unwrap();
        assert_eq!(**clone, "a");
        assert_eq!(*value, "b");

        // `Weak` references also prevent mutation
        let weak = Arc::downgrade(&value);
        let reflect = &mut value as &mut dyn Reflect;
        assert!(reflect.pointer_mut().unwrap().get_mut().is_none());
        drop(weak);
        reflect
            .pointer_mut()
            .unwrap()
            .get_mut()
            .unwrap()
            .downcast_mut::<String>()
            .unwrap()
            .push('c');
        assert_eq!(*value, "bc");
    }

    #[test]
    fn arc_wrap() {
        let Kind::Pointer(pointer_type) = Arc::<String>::TYPE.kind() else {
            panic!("Expected pointer kind");
        };
        let wrapped = pointer_type.wrap(Box::new(String::from("a"))).unwrap();
        assert_eq!(
            wrapped
                .downcast_ref::<Arc<String>>()
                .map(|value| value.as_str()),
            Some("a")
        );
        assert!(pointer_type.wrap(Box::new(1u32)).is_err());
    }

    trait Shape: Reflect + Send + Sync {
        fn area(&self) -> f32;
    }

    impl_reflect_pointee!(dyn Shape);

    #[derive(Reflect, Debug, PartialEq)]
    struct Square(f32);

    impl Shape for Square {
        fn area(&self) -> f32 {
            self.0 * self.0
        }
    }

    #[derive(Reflect)]
    struct Scene {
        shape: Arc<dyn Shape>,
    }

    #[test]
    fn arc_dyn() {
        assert_eq!(Arc::<dyn Shape>::TYPE.type_path().short(), "Arc<dyn Shape>");
        let Kind::Pointer(pointer_type) = Arc::<dyn Shape>::TYPE.kind() else {
            panic!("Expected pointer kind");
        };
        assert!(pointer_type.pointee().is_none());

        let mut scene = Scene {
            shape: Arc::new(Square(2.0)),
        };
        let reflect = &mut scene as &mut dyn Reflect;
        let mut struct_mut = reflect.struct_mut().unwrap();
        let mut pointer = struct_mut.field_mut(0).unwrap().pointer_mut().unwrap();
        assert_eq!(pointer.get().downcast_ref(), Some(&Square(2.0)));
        pointer
            .get_mut()
            .unwrap()
            .downcast_mut::<Square>()
            .unwrap()
            .0 = 3.0;
        // Trait objects can't be created from a `Box<dyn Reflect>`
        assert!(pointer.set(Box::new(Square(4.0))).is_err());
        assert_eq!(scene.shape.area(), 9.0);

        // Unlike `Box<dyn Shape>`, `Arc<dyn Shape>` can be cloned, because the clone shares the
        // value
        let reflect = &scene as &dyn Reflect;
        assert_eq!(format!("{reflect:?}"), "Scene { shape: Square(3.0) }");
        let clone = reflect.reflect_clone().unwrap();
        let clone = clone.downcast_ref::<Scene>().unwrap();
        assert!(Arc::ptr_eq(&scene.shape, &clone.shape));

        let mut type_registry = TypeRegistry::empty();
        type_registry.register::<Scene>();
        assert!(type_registry.contains(TypeId::of::<Arc<dyn Shape>>()));
        assert!(!type_registry.contains(TypeId::of::<Square>()));
    }

    #[test]
    fn arc_nested() {
        let value: Arc<Box<u32>> = Arc::new(Box::new(1));
        let reflect = &value as &dyn Reflect;
        assert_eq!(reflect.ty().type_path().short(), "Arc<Box<u32>>");
        let inner = reflect.pointer_ref().unwrap().get();
        assert_eq!(
            inner.pointer_ref().unwrap().get().downcast_ref(),
            Some(&1u32)
        );
    }
}
