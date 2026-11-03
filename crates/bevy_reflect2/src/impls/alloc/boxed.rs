use crate::{
    kind::Kind, kind::Pointer, kind::PointerVTable, kind::ReflectPointee,
    type_path::GenericTypePath, type_path::GenericTypePathCell, type_path::TypePath,
    type_registry::GetTypeRegistration, type_registry::TypeRegistration,
    type_registry::TypeRegistry, type_vtable::TypeVTable, Reflect, Type, TypeRef, Typed,
};
use alloc::boxed::Box;
use bevy_ptr::PtrMut;
use core::{alloc::Layout, any::TypeId, ptr::NonNull};

// SAFETY: `Box<T>` is described by `THIN_BOX_VTABLE` if `T` is sized (`ReflectPointee::TYPE` is
// only `Some` for sized types), and by `box_vtable::<T>` otherwise, which are valid for `Box<T>`.
// The layout and vtable are derived from `Self`.
unsafe impl<T: ?Sized + ReflectPointee> Typed for Box<T> {
    // SAFETY: see above. `ReflectPointee::TYPE` is either `None` or the type of `T`, the boxed
    // value.
    const TYPE: &'static Type = &unsafe {
        // The functions of the vtable that isn't used are not generated, so boxes of sized types
        // don't generate any functions for their pointer vtable
        let vtable = if T::TYPE.is_some() {
            THIN_BOX_VTABLE
        } else {
            box_vtable::<T>()
        };
        Type::new(
            TypeId::of::<Self>(),
            Kind::Pointer(Pointer::new(TypeRef(Type::of::<Self>), T::TYPE, vtable)),
            Some(Layout::new::<Self>()),
            TypeVTable::new::<Self>(),
            TypePath::generic::<Self>("alloc::boxed", "Box"),
        )
    };
}

/// The [`PointerVTable`] of `Box<T>` for every sized `T`.
///
/// A `Box<T>` of a sized `T` is guaranteed to be represented as a single (thin) non-null pointer
/// to a `T` allocated with the global allocator using the layout of `T` (or a dangling pointer, if
/// `T` is zero-sized). That makes every `Box<T>` the same, except for the type of the value, so
/// these functions are shared by all of them, using [`Pointer::pointee`] instead of `T`.
///
/// Every function requires that the [`Pointer`] it is given describes a `Box<T>` of a sized `T`,
/// with `Some` [`Pointer::pointee`].
const THIN_BOX_VTABLE: PointerVTable = PointerVTable {
    cast_pointee: thin_box_cast_pointee,
    set: thin_box_set,
    wrap: thin_box_wrap,
};

/// Returns the type of the value in a box described by [`THIN_BOX_VTABLE`].
fn thin_box_pointee(pointer: &Pointer) -> &'static Type {
    pointer
        .pointee()
        .expect("boxes of sized types have a statically known pointee")
        .get()
}

/// # Safety
///
/// See [`THIN_BOX_VTABLE`] and [`PointerVTable`].
unsafe fn thin_box_cast_pointee(pointer: &'static Pointer, ptr: *const u8) -> *const dyn Reflect {
    // SAFETY: the caller guarantees that `ptr` is a `Box<T>` of a sized `T`, which is represented
    // as a single pointer to the `T`. The pointer read from it can be written to whenever the box
    // is exclusively borrowed, since the box owns its value.
    let data = unsafe { ptr.cast::<*const u8>().read() };
    thin_box_pointee(pointer).as_reflect_ptr(data)
}

/// # Safety
///
/// See [`THIN_BOX_VTABLE`] and [`PointerVTable`].
unsafe fn thin_box_set(
    pointer: &'static Pointer,
    ptr: PtrMut<'_>,
    value: Box<dyn Reflect>,
) -> Result<(), Box<dyn Reflect>> {
    let pointee = thin_box_pointee(pointer);
    if value.ty().type_id() != pointee.type_id() {
        return Err(value);
    }
    // `value` is a `T`, so its allocation has the layout of `T`, and its data pointer is a valid
    // `Box<T>`
    let data = Box::into_raw(value).cast::<u8>();
    // SAFETY: the caller guarantees that `ptr` is a `Box<T>` of a sized `T`, and `data` is a valid
    // `Box<T>`. The previous value is moved out, and is dropped below.
    let previous = unsafe {
        ptr.as_ptr()
            .cast::<NonNull<u8>>()
            .replace(NonNull::new_unchecked(data))
    };
    // SAFETY: `previous` was owned by a `Box<T>`, so it is a valid `T` allocated with the global
    // allocator using the layout of `T`
    drop(unsafe { pointee.box_from_raw(previous) });
    Ok(())
}

fn thin_box_wrap(
    pointer: &'static Pointer,
    value: Box<dyn Reflect>,
) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
    if value.ty().type_id() != thin_box_pointee(pointer).type_id() {
        return Err(value);
    }
    let ty = pointer.ty().get();
    // Allocate before taking ownership of `value`, so it is dropped normally if this panics
    let dst = ty.alloc().expect("boxes are sized");
    // `value` is a `T`, so its allocation has the layout of `T`, and its data pointer is a valid
    // `Box<T>`
    let data = Box::into_raw(value).cast::<u8>();
    // SAFETY: `dst` is a new allocation of a `Box<T>`, which is represented as a pointer
    unsafe { dst.as_ptr().cast::<*mut u8>().write(data) };
    // SAFETY: `dst` was allocated with `Type::alloc`, and was initialized above
    Ok(unsafe { ty.box_from_raw(dst) })
}

/// Returns the [`PointerVTable`] of `Box<T>` for any `T` (ex: trait objects).
const fn box_vtable<T: ?Sized + ReflectPointee>() -> PointerVTable {
    PointerVTable {
        cast_pointee: box_cast_pointee::<T>,
        set: box_set::<T>,
        wrap: box_wrap::<T>,
    }
}

/// # Safety
///
/// `ptr` must be a valid `Box<T>`.
unsafe fn box_cast_pointee<T: ?Sized + ReflectPointee>(
    _pointer: &'static Pointer,
    ptr: *const u8,
) -> *const dyn Reflect {
    // The value is accessed through a raw place, rather than by dereferencing the box through a
    // reference, so the returned pointer can be written to whenever the box is exclusively
    // borrowed.
    //
    // SAFETY: the caller guarantees that `ptr` is a valid `Box<T>`
    let value = unsafe { &raw mut **ptr.cast_mut().cast::<Box<T>>() };
    T::cast_reflect(value)
}

/// # Safety
///
/// `ptr` must be a valid `Box<T>`.
unsafe fn box_set<T: ?Sized + ReflectPointee>(
    _pointer: &'static Pointer,
    ptr: PtrMut<'_>,
    value: Box<dyn Reflect>,
) -> Result<(), Box<dyn Reflect>> {
    // SAFETY: the caller guarantees that `ptr` is a valid `Box<T>`
    *unsafe { ptr.deref_mut::<Box<T>>() } = T::from_reflect_box(value)?;
    Ok(())
}

fn box_wrap<T: ?Sized + ReflectPointee>(
    _pointer: &'static Pointer,
    value: Box<dyn Reflect>,
) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
    Ok(Box::new(T::from_reflect_box(value)?))
}

impl<T: ?Sized + ReflectPointee> GenericTypePath for Box<T> {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            alloc::format!("alloc::boxed::Box<{}>", T::type_path())
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert(TypeId::of::<Self>(), || {
            alloc::format!("Box<{}>", T::short_type_path())
        })
    }
}

impl<T: ?Sized + ReflectPointee> GetTypeRegistration for Box<T> {
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
        self as bevy_reflect2, builtin::ReflectCloneErrorReason, impl_reflect_pointee,
        type_registry::TypeRegistry, Reflect, Typed,
    };
    use alloc::{boxed::Box, format, string::String};
    use core::any::TypeId;

    #[test]
    fn box_sized() {
        assert_eq!(
            Box::<u32>::TYPE.type_path().full(),
            "alloc::boxed::Box<u32>"
        );
        assert_eq!(Box::<u32>::TYPE.type_path().short(), "Box<u32>");
        let pointer_type = Box::<u32>::TYPE.kind();
        let crate::kind::Kind::Pointer(pointer_type) = pointer_type else {
            panic!("Expected pointer kind");
        };
        assert_eq!(
            pointer_type.pointee().unwrap().get().type_id(),
            TypeId::of::<u32>()
        );

        let mut value = Box::new(1u32);
        let reflect = &mut value as &mut dyn Reflect;
        assert_eq!(
            reflect.pointer_ref().unwrap().get().downcast_ref(),
            Some(&1u32)
        );
        *reflect
            .pointer_mut()
            .unwrap()
            .get_mut()
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

        let clone = reflect.reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<Box<u32>>(), Some(&Box::new(3u32)));
        assert_eq!(reflect.reflect_partial_eq(&*clone).ok(), Some(true));
        assert_eq!(
            reflect.reflect_partial_cmp(&Box::new(4u32)).unwrap(),
            Some(core::cmp::Ordering::Less)
        );
        assert_eq!(reflect.reflect_hash().ok(), clone.reflect_hash().ok());
        assert_ne!(
            reflect.reflect_hash().unwrap(),
            (&3u32 as &dyn Reflect).reflect_hash().unwrap()
        );
        assert_eq!(*value, 3);
    }

    trait Shape: Reflect {
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
        shape: Box<dyn Shape>,
    }

    #[test]
    fn box_dyn() {
        assert_eq!(
            Box::<dyn Shape>::TYPE.type_path().full(),
            "alloc::boxed::Box<dyn bevy_reflect2::impls::alloc::boxed::tests::Shape>"
        );
        assert_eq!(Box::<dyn Shape>::TYPE.type_path().short(), "Box<dyn Shape>");
        let crate::kind::Kind::Pointer(pointer_type) = Box::<dyn Shape>::TYPE.kind() else {
            panic!("Expected pointer kind");
        };
        assert!(pointer_type.pointee().is_none());

        let mut scene = Scene {
            shape: Box::new(Square(2.0)),
        };
        let reflect = &mut scene as &mut dyn Reflect;
        let mut struct_mut = reflect.struct_mut().unwrap();
        let mut pointer = struct_mut.field_mut(0).unwrap().pointer_mut().unwrap();
        assert_eq!(pointer.get().downcast_ref(), Some(&Square(2.0)));
        pointer.get_mut().downcast_mut::<Square>().unwrap().0 = 3.0;
        // Trait objects can't be created from a `Box<dyn Reflect>`
        assert!(pointer.set(Box::new(Square(4.0))).is_err());
        assert_eq!(scene.shape.area(), 9.0);

        let reflect = &scene as &dyn Reflect;
        assert_eq!(format!("{reflect:?}"), "Scene { shape: Square(3.0) }");
        let error = reflect.reflect_clone().unwrap_err();
        assert_eq!(error.ty.type_id(), TypeId::of::<Box<dyn Shape>>());
        assert!(matches!(error.reason, ReflectCloneErrorReason::WrapFailed));

        let mut type_registry = TypeRegistry::empty();
        type_registry.register::<Scene>();
        assert!(type_registry.contains(TypeId::of::<Box<dyn Shape>>()));
        assert!(!type_registry.contains(TypeId::of::<Square>()));
    }

    #[test]
    fn box_owned_pointee() {
        // The previous value is dropped when replaced, and clones own their value
        let mut value = Box::new(String::from("a"));
        let reflect = &mut value as &mut dyn Reflect;
        reflect
            .pointer_mut()
            .unwrap()
            .set(Box::new(String::from("b")))
            .unwrap();
        let clone = reflect.reflect_clone().unwrap();
        reflect
            .pointer_mut()
            .unwrap()
            .get_mut()
            .downcast_mut::<String>()
            .unwrap()
            .push('c');
        assert_eq!(*value, "bc");
        assert_eq!(
            clone
                .downcast_ref::<Box<String>>()
                .map(|value| value.as_str()),
            Some("b")
        );
    }

    #[test]
    fn box_zero_sized_pointee() {
        let mut value = Box::new(());
        let reflect = &mut value as &mut dyn Reflect;
        assert!(reflect.pointer_ref().unwrap().get().is::<()>());
        reflect.pointer_mut().unwrap().set(Box::new(())).unwrap();
        let clone = reflect.reflect_clone().unwrap();
        assert!(clone.is::<Box<()>>());
    }

    #[test]
    fn box_nested() {
        let value: Box<Box<u32>> = Box::new(Box::new(1));
        let reflect = &value as &dyn Reflect;
        assert_eq!(reflect.ty().type_path().short(), "Box<Box<u32>>");
        let inner = reflect.pointer_ref().unwrap().get();
        assert_eq!(
            inner.pointer_ref().unwrap().get().downcast_ref(),
            Some(&1u32)
        );
        let clone = reflect.reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<Box<Box<u32>>>(), Some(&value));
    }
}
