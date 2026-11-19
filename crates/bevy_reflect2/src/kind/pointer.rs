use crate::{
    type_registry::GetTypeRegistration, type_registry::TypeRegistry, Reflect, Type, TypeRef, Typed,
};
use alloc::boxed::Box;
use bevy_ptr::{Ptr, PtrMut};

/// Describes a pointer to a single value (ex: `Box<T>`). The value (the "pointee") is either a
/// statically known type (ex: `Box<u32>`) or a trait object (ex: `Box<dyn MyTrait>`), in which
/// case its type is only known at runtime, from the value itself.
///
/// Unsafe code relies on the [`PointerVTable`] being valid for the described type, so this can
/// only be constructed using [`Pointer::new`].
#[derive(Debug)]
pub struct Pointer {
    ty: TypeRef,
    pointee: Option<TypeRef>,
    vtable: PointerVTable,
}

impl Pointer {
    /// # Safety
    ///
    /// - `ty` must be the pointer type being described.
    /// - If `pointee` is `Some`, it must be the type of every value the pointer points to.
    /// - The functions in `vtable` must be valid for values of `ty`, when given this [`Pointer`]
    ///   (see [`PointerVTable`]).
    pub const unsafe fn new(ty: TypeRef, pointee: Option<TypeRef>, vtable: PointerVTable) -> Self {
        Self {
            ty,
            pointee,
            vtable,
        }
    }

    /// The pointer type.
    #[inline]
    pub const fn ty(&self) -> TypeRef {
        self.ty
    }

    /// The type of the value the pointer points to, or `None` if it is only known at runtime (ex:
    /// trait objects like `Box<dyn MyTrait>`).
    #[inline]
    pub const fn pointee(&self) -> Option<TypeRef> {
        self.pointee
    }

    /// The type-erased functions used to access the pointer.
    #[inline]
    pub const fn vtable(&self) -> &PointerVTable {
        &self.vtable
    }

    /// Creates a new pointer of this type that owns `value`. Returns `value` in an `Err` if the
    /// pointer can't store it (ex: it has the wrong type, or the pointee is a trait object, which
    /// can't be created from a `Box<dyn Reflect>`).
    #[inline]
    pub fn wrap(
        &'static self,
        value: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        (self.vtable.wrap)(self, value)
    }
}

/// Type-erased functions for a [`Pointer`].
///
/// Each function is given the [`Pointer`] that it was registered in, which lets a single function
/// support many pointer types (ex: every `Box<T>` with a sized `T`). Every function that takes a
/// pointer to the pointer type requires that it is a valid, initialized value of [`Pointer::ty`].
///
/// Values passed into the pointer are type checked: if a value can't be stored, it is returned in
/// an `Err`.
#[derive(Debug, Clone, Copy)]
pub struct PointerVTable {
    /// Casts a pointer to a value of [`Pointer::ty`] to a pointer to the value it points to, using
    /// the value's `Reflect` implementation (like [`TypeVTable::cast_reflect`](crate::type_vtable::TypeVTable)).
    ///
    /// The returned pointer is valid for reads for as long as the pointer value is borrowed. If
    /// [`Pointer::pointee`] is `Some`, it points to a value of that type.
    pub cast_pointee: unsafe fn(&'static Pointer, *const u8) -> *const dyn Reflect,
    /// Like [`PointerVTable::cast_pointee`], but the returned pointer can be written to for as long
    /// as the pointer value is exclusively borrowed. Returns `None` if the value can't be mutated
    /// through this pointer (ex: an `Arc<T>` whose value is shared with other `Arc`s or `Weak`s).
    pub cast_pointee_mut: unsafe fn(&'static Pointer, PtrMut<'_>) -> Option<*mut dyn Reflect>,
    /// Replaces the value the pointer points to. Pointers that share their value (ex: `Arc<T>`)
    /// point to the new value instead of modifying the shared value.
    pub set:
        unsafe fn(&'static Pointer, PtrMut<'_>, Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>,
    /// Returns a new value of [`Pointer::ty`] that owns the given value, or returns the given
    /// value unchanged in an `Err`.
    pub wrap: fn(&'static Pointer, Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>>,
}

/// A type that can be reflected behind a pointer (ex: the `T` in `Box<T>`).
///
/// This is implemented for every [`Typed`] type. Trait objects of traits that have [`Reflect`] as
/// a supertrait can implement it using [`impl_reflect_pointee`](crate::impl_reflect_pointee).
///
/// # Safety
///
/// - [`ReflectPointee::TYPE`] must be `Some(TypeRef)` of `Self`, or `None`. It must be `None` if
///   `Self` is unsized (pointer types rely on this to treat pointers to it as thin pointers).
/// - [`ReflectPointee::cast_reflect`] must return `ptr`, with the metadata of `Self`'s `Reflect`
///   implementation.
/// - [`ReflectPointee::from_reflect_box`] must return `value` as a `Box<Self>`, or return it
///   unchanged in an `Err`.
pub unsafe trait ReflectPointee: 'static {
    /// The type of every value of `Self`, or `None` if it is only known at runtime (ex: trait
    /// objects).
    const TYPE: Option<TypeRef>;

    /// Casts a pointer to `Self` to a pointer to its `Reflect` implementation (ex: by upcasting
    /// a trait object). Pointer types use this to access their value without creating references.
    fn cast_reflect(ptr: *const Self) -> *const dyn Reflect;
    /// Converts `value` to a `Box<Self>`, or returns it unchanged in an `Err` if it can't be
    /// converted.
    fn from_reflect_box(value: Box<dyn Reflect>) -> Result<Box<Self>, Box<dyn Reflect>>;
    /// The full type path of `Self` (see [`TypePath::full`](crate::type_path::TypePath::full)).
    fn type_path() -> &'static str;
    /// The short type path of `Self` (see [`TypePath::short`](crate::type_path::TypePath::short)).
    fn short_type_path() -> &'static str;
    /// Registers `Self`, if its type is statically known.
    fn register_type_dependencies(_registry: &mut TypeRegistry) {}
}

// SAFETY: `TYPE` is the type of `T`, `cast_reflect` returns `ptr`, and `from_reflect_box` only
// succeeds if the value is a `T`
unsafe impl<T: Typed + GetTypeRegistration> ReflectPointee for T {
    const TYPE: Option<TypeRef> = Some(TypeRef(Type::of::<T>));

    fn cast_reflect(ptr: *const Self) -> *const dyn Reflect {
        ptr
    }

    fn from_reflect_box(value: Box<dyn Reflect>) -> Result<Box<Self>, Box<dyn Reflect>> {
        value.downcast::<T>()
    }

    fn type_path() -> &'static str {
        T::TYPE.type_path().full()
    }

    fn short_type_path() -> &'static str {
        T::TYPE.type_path().short()
    }

    fn register_type_dependencies(registry: &mut TypeRegistry) {
        registry.register::<T>();
    }
}

/// Implements [`ReflectPointee`] for a trait object, which makes pointers to it reflectable (ex:
/// `Box<dyn MyTrait>`). The trait must have [`Reflect`] as a supertrait.
///
/// The value behind the pointer can be accessed (and mutated) through [`PointerRef`] /
/// [`PointerMut`], but it can't be replaced, and the pointer can't be cloned structurally, since a
/// `Box<dyn Reflect>` can't be converted to a `Box<dyn MyTrait>`.
///
/// # Example
///
/// ```
/// # use bevy_reflect2::{impl_reflect_pointee, Reflect};
/// trait Shape: Reflect {
///     fn area(&self) -> f32;
/// }
///
/// impl_reflect_pointee!(dyn Shape);
///
/// #[derive(Reflect)]
/// struct Square(f32);
///
/// impl Shape for Square {
///     fn area(&self) -> f32 {
///         self.0 * self.0
///     }
/// }
///
/// let shape: Box<dyn Shape> = Box::new(Square(2.0));
/// let reflect = &shape as &dyn Reflect;
/// let pointee = reflect.pointer_ref().unwrap().get();
/// assert_eq!(pointee.downcast_ref::<Square>().unwrap().0, 2.0);
/// ```
#[macro_export]
macro_rules! impl_reflect_pointee {
    ($ty:ty) => {
        // SAFETY: `TYPE` is `None`, `cast_reflect` returns `ptr` (upcast to `dyn Reflect`), and
        // `from_reflect_box` never succeeds
        unsafe impl $crate::kind::ReflectPointee for $ty {
            const TYPE: ::core::option::Option<$crate::TypeRef> = ::core::option::Option::None;

            fn cast_reflect(ptr: *const Self) -> *const dyn $crate::Reflect {
                ptr
            }

            fn from_reflect_box(
                value: $crate::__macro_exports::Box<dyn $crate::Reflect>,
            ) -> ::core::result::Result<
                $crate::__macro_exports::Box<Self>,
                $crate::__macro_exports::Box<dyn $crate::Reflect>,
            > {
                ::core::result::Result::Err(value)
            }

            fn type_path() -> &'static str {
                ::core::any::type_name::<Self>()
            }

            fn short_type_path() -> &'static str {
                static CELL: $crate::type_path::GenericTypePathCell =
                    $crate::type_path::GenericTypePathCell::new();
                CELL.get_or_insert(::core::any::TypeId::of::<Self>(), || {
                    $crate::__macro_exports::format!(
                        "{}",
                        $crate::__macro_exports::ShortName::of::<Self>()
                    )
                })
            }
        }
    };
}

/// A reference to a reflected [`Pointer`] value (ex: a `&Box<T>`), which provides access to the
/// value it points to.
pub struct PointerRef<'a> {
    // SAFETY NOTE: this should only ever be constructed with PointerRef::new
    ptr: Ptr<'a>,
    // SAFETY NOTE: this should only ever be constructed with PointerRef::new
    pointer_type: &'static Pointer,
}

impl<'a> PointerRef<'a> {
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in
    /// `pointer_type`
    pub unsafe fn new(ptr: Ptr<'a>, pointer_type: &'static Pointer) -> Self {
        Self { ptr, pointer_type }
    }

    /// The [`Pointer`] that describes this value's type.
    #[inline]
    pub fn pointer_type(&self) -> &'static Pointer {
        self.pointer_type
    }

    /// Returns the value the pointer points to.
    #[inline]
    pub fn get(&self) -> &'a dyn Reflect {
        let pointee = self.pointer_type.vtable.cast_pointee;
        // SAFETY: `PointerRef::new` guarantees that `ptr` matches `pointer_type`, and it is
        // borrowed for `'a`
        unsafe { &*pointee(self.pointer_type, self.ptr.as_ptr()) }
    }
}

/// A mutable reference to a reflected [`Pointer`] value (ex: a `&mut Box<T>`), which provides
/// (mutable) access to the value it points to.
pub struct PointerMut<'a> {
    // SAFETY NOTE: this should only ever be constructed with PointerMut::new
    ptr: PtrMut<'a>,
    // SAFETY NOTE: this should only ever be constructed with PointerMut::new
    pointer_type: &'static Pointer,
}

impl<'a> PointerMut<'a> {
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in
    /// `pointer_type`
    pub unsafe fn new(ptr: PtrMut<'a>, pointer_type: &'static Pointer) -> Self {
        Self { ptr, pointer_type }
    }

    /// Returns a read-only view of this pointer.
    #[inline]
    pub fn as_ref(&self) -> PointerRef<'_> {
        PointerRef {
            ptr: self.ptr.as_ref(),
            pointer_type: self.pointer_type,
        }
    }

    /// The [`Pointer`] that describes this value's type.
    #[inline]
    pub fn pointer_type(&self) -> &'static Pointer {
        self.pointer_type
    }

    /// Returns the value the pointer points to.
    #[inline]
    pub fn get(&self) -> &dyn Reflect {
        self.as_ref().get()
    }

    /// Returns the value the pointer points to, or `None` if it can't be mutated through this
    /// pointer (ex: an `Arc<T>` whose value is shared with other `Arc`s or `Weak`s).
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut dyn Reflect> {
        let pointee = self.pointer_type.vtable.cast_pointee_mut;
        // SAFETY: `PointerMut::new` guarantees that `ptr` matches `pointer_type`, and it is
        // exclusively borrowed for the lifetime of `self`, so the value can be written to
        unsafe { pointee(self.pointer_type, self.ptr.reborrow()).map(|value| &mut *value) }
    }

    /// Replaces the value the pointer points to with `value`. Returns `value` if the pointer can't
    /// store it (ex: it has the wrong type, or the pointee is a trait object, which can't be
    /// created from a `Box<dyn Reflect>`).
    ///
    /// Pointers that share their value (ex: `Arc<T>`) point to the new value instead of modifying
    /// the shared value.
    #[inline]
    pub fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        // SAFETY: `PointerMut::new` guarantees that `ptr` matches `pointer_type`
        unsafe { (self.pointer_type.vtable.set)(self.pointer_type, self.ptr.reborrow(), value) }
    }
}
