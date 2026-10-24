use bevy_ptr::{Ptr, PtrMut};

use crate::{
    builtin::move_box_into,
    kind::{Field, Kind, UncoveredBytes},
    reflect::Reflect,
    TypePath, TypeRef,
};
use alloc::{
    alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error},
    boxed::Box,
    vec::Vec,
};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    ptr::{self, NonNull},
};

/// Describes a Rust type.
///
/// Unsafe code relies on this information being accurate (ex: field offsets, layout, and vtable
/// functions), so it can only be constructed using [`Type::new`].
pub struct Type {
    type_id: TypeId,
    kind: Kind,
    layout: Option<Layout>,
    vtable: TypeVTable,
    type_path: TypePath,
}

impl Type {
    /// Creates a new [`Type`] description.
    ///
    /// # Safety
    ///
    /// Every argument must accurately describe the type identified by `type_id`:
    /// - `kind` must match the type's shape (ex: [`Struct`](crate::Struct) field offsets and types).
    /// - `layout` must be the layout of the type, or `None` if the type is unsized.
    /// - The functions in `vtable` must be valid for pointers to the type.
    pub const unsafe fn new(
        type_id: TypeId,
        kind: Kind,
        layout: Option<Layout>,
        vtable: TypeVTable,
        type_path: TypePath,
    ) -> Self {
        Self {
            type_id,
            kind,
            layout,
            vtable,
            type_path,
        }
    }

    /// Returns the [`Type`] of the given type `T`.
    pub const fn of<T: Typed>() -> &'static Type {
        T::TYPE
    }

    /// Returns the [`TypeId`] of this type.
    #[inline]
    pub const fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the [`Kind`] of this type.
    #[inline]
    pub const fn kind(&self) -> &Kind {
        &self.kind
    }

    /// Returns the [`Layout`] of this type, if it has one (ex: [`Sized`] types have a layout).
    #[inline]
    pub const fn layout(&self) -> Option<Layout> {
        self.layout
    }

    /// Returns the [`TypeVTable`] of this type.
    #[inline]
    pub const fn vtable(&self) -> &TypeVTable {
        &self.vtable
    }

    /// Returns the [`TypePath`] of this type.
    #[inline]
    pub const fn type_path(&self) -> &TypePath {
        &self.type_path
    }

    /// Allocates uninitialized memory for a value of this type. Returns `None` if the type is unsized.
    ///
    /// Zero-sized types do not allocate, and instead return a dangling, well-aligned pointer.
    pub fn alloc(&self) -> Option<NonNull<u8>> {
        self.alloc_with(alloc)
    }

    /// Allocates zeroed memory for a value of this type. Returns `None` if the type is unsized.
    ///
    /// Zero-sized types do not allocate, and instead return a dangling, well-aligned pointer.
    pub fn alloc_zeroed(&self) -> Option<NonNull<u8>> {
        self.alloc_with(alloc_zeroed)
    }

    fn alloc_with(&self, alloc_fn: unsafe fn(Layout) -> *mut u8) -> Option<NonNull<u8>> {
        let layout = self.layout?;
        if layout.size() == 0 {
            // Alignment is never zero, so this is always `Some`.
            return NonNull::new(ptr::without_provenance_mut(layout.align()));
        }
        // SAFETY: `layout` has a non-zero size
        let ptr = unsafe { alloc_fn(layout) };
        Some(NonNull::new(ptr).unwrap_or_else(|| handle_alloc_error(layout)))
    }

    /// Deallocates memory allocated with [`Type::alloc`] or [`Type::alloc_zeroed`], without dropping
    /// its contents.
    ///
    /// # Safety
    ///
    /// `ptr` must have been allocated by this type's [`Type::alloc`] or [`Type::alloc_zeroed`], and
    /// must not be used afterwards.
    pub(crate) unsafe fn dealloc(&self, ptr: NonNull<u8>) {
        let Some(layout) = self.layout else {
            return;
        };
        if layout.size() != 0 {
            // SAFETY: the caller guarantees that `ptr` was allocated with the global allocator using
            // this type's layout, which has a non-zero size
            unsafe { dealloc(ptr.as_ptr(), layout) };
        }
    }

    /// Returns a default instance of this type. This is generally a [`Default`] implementation, provided
    /// via `#[reflect(Default)]`.
    pub fn default(&self) -> Option<Box<dyn Reflect>> {
        let set_default = self.vtable.set_default?;
        let ptr = self.alloc()?;
        // SAFETY: `ptr` is a new allocation using the reflected layout for this type
        unsafe { (set_default)(ptr.as_ptr()) };
        // SAFETY: `ptr` is a fully initialized allocation of this type
        Some(unsafe { self.box_from_raw(ptr) })
    }

    /// Allocates a new value of this type, initializing each field of the struct with `func`.
    ///
    /// Bytes that are not covered by a field are left uninitialized if they are padding, or zeroed
    /// before `func` is called if the struct requires it (see
    /// [`UncoveredBytes`](crate::UncoveredBytes)).
    ///
    /// [Ignored fields](crate::Struct::ignored_fields) are initialized with their
    /// [`default`](crate::IgnoredFieldVTable::default) function (registered with
    /// `#[reflect(default)]`) after `func` initializes the reflected fields.
    ///
    /// Returns `None` if this type is not a [`Kind::Struct`], is unsized, or has an ignored field
    /// without a `default` function.
    ///
    /// # Safety
    ///
    /// `func` must initialize every pointer to a valid value that matches the type described in each [`Field`].
    /// The pointers may point to uninitialized memory, so values must be written without reading or
    /// dropping the previous value (ex: with [`ptr::write`]), rather than assigned through a
    /// reference.
    pub unsafe fn initialize_struct(
        &self,
        mut func: impl FnMut(&Field, PtrMut),
    ) -> Option<Box<dyn Reflect>> {
        let Kind::Struct(struct_type) = &self.kind else {
            return None;
        };
        // Ignored fields without a default would be left uninitialized
        let ignored_fields = struct_type.ignored_fields();
        if ignored_fields
            .iter()
            .any(|field| field.vtable.default.is_none())
        {
            return None;
        }
        let ptr = match struct_type.uncovered_bytes() {
            UncoveredBytes::Padding => self.alloc()?,
            UncoveredBytes::Zeroed => self.alloc_zeroed()?,
        };
        for field in struct_type.fields() {
            // SAFETY: `ptr` is a unique allocation of this type, and `Type::new` guarantees that
            // `field.offset` is in bounds of it
            let field_ptr = unsafe { PtrMut::new(ptr).byte_add(field.offset) };
            (func)(field, field_ptr);
        }
        for field in ignored_fields {
            if let Some(default) = field.vtable.default {
                // SAFETY: `Struct::new` guarantees that `field.offset` is the location of the field
                // in this allocation, and that `default` is valid for the field's type
                unsafe { default(ptr.as_ptr().add(field.offset)) };
            }
        }

        // SAFETY: the caller guarantees that every reflected field is now initialized, and every
        // ignored field was initialized above. `Struct::new` guarantees that these are all of the
        // struct's fields, and that every other byte is either padding (which doesn't need to be
        // initialized), or was zeroed above, which `Struct::new` guarantees is valid.
        Some(unsafe { self.box_from_raw(ptr) })
    }

    /// Constructs a new value of this struct type from the values of its reflected fields, in the
    /// order of [`Struct::fields`](crate::Struct::fields). [Ignored
    /// fields](crate::Struct::ignored_fields) are initialized with their default (see
    /// [`Type::initialize_struct`]).
    ///
    /// Returns `None` if this type is not a [`Kind::Struct`], the number of values doesn't match
    /// the number of fields, a value has the wrong type, or an ignored field has no default.
    pub fn construct_struct(
        &self,
        values: impl IntoIterator<Item = Box<dyn Reflect>>,
    ) -> Option<Box<dyn Reflect>> {
        let Kind::Struct(struct_type) = &self.kind else {
            return None;
        };
        let values = values.into_iter().collect::<Vec<_>>();
        if !types_match(&values, struct_type.fields().iter().map(|field| field.ty)) {
            return None;
        }
        let mut values = values.into_iter();
        // SAFETY: every field is initialized by moving the value of its type into it (checked
        // above), in the order of `Struct::fields`
        unsafe {
            self.initialize_struct(|_, field_ptr| {
                let value = values.next().expect("there is a value for each field");
                move_box_into(value, field_ptr.as_ptr());
            })
        }
    }

    /// Constructs a new value of this array type from its items.
    ///
    /// Returns `None` if this type is not a [`Kind::Array`], the number of items doesn't match the
    /// length of the array, or an item has the wrong type.
    pub fn construct_array(
        &self,
        items: impl IntoIterator<Item = Box<dyn Reflect>>,
    ) -> Option<Box<dyn Reflect>> {
        let Kind::Array(array_type) = &self.kind else {
            return None;
        };
        let items = items.into_iter().collect::<Vec<_>>();
        if !types_match(&items, (0..array_type.len()).map(|_| array_type.item())) {
            return None;
        }
        let dst = self.alloc()?;
        for (index, item) in items.into_iter().enumerate() {
            // SAFETY: `Array::new` guarantees that the item at `index` is at `index * stride` in
            // an array of this type, and `item` was checked to have the item type
            unsafe { move_box_into(item, dst.as_ptr().add(index * array_type.stride())) };
        }
        // SAFETY: `dst` was allocated with `Type::alloc`, and every item was initialized above.
        // Arrays have no bytes other than their items.
        Some(unsafe { self.box_from_raw(dst) })
    }

    /// # Safety
    ///
    /// `ptr` must be a fully initialized value of this type, allocated with the global allocator
    /// using this type's layout, or dangling if the type is zero-sized (ex: using [`Type::alloc`],
    /// or by a `Box` of this type).
    pub(crate) unsafe fn box_from_raw(&self, ptr: NonNull<u8>) -> Box<dyn Reflect> {
        let reflect_ptr = (self.vtable.cast_reflect)(ptr.as_ptr()).cast_mut();
        // SAFETY: the caller guarantees that `ptr` is an initialized value allocated with the global
        // allocator using this type's layout (or is dangling for zero-sized types)
        unsafe { Box::from_raw(reflect_ptr) }
    }

    /// Converts the `ptr` to a *const dyn Reflect pointer, using the Reflect implementation associated
    /// with this type.
    pub fn as_reflect_ptr(&self, ptr: *const u8) -> *const dyn Reflect {
        (self.vtable.cast_reflect)(ptr)
    }

    /// Returns `value` as a reflected value if it is a value of this type (its [`TypeId`] matches
    /// [`Type::type_id`]).
    pub fn reflect_any<'a>(&self, value: &'a dyn Any) -> Option<&'a dyn Reflect> {
        if value.type_id() != self.type_id {
            return None;
        }
        let ptr = NonNull::from(value).cast::<u8>();
        // SAFETY: `value` is a valid value of this type (checked above), borrowed for `'a`
        Some(unsafe { self.as_reflect(Ptr::new(ptr)) })
    }

    /// Returns `value` as a mutable reflected value if it is a value of this type (its [`TypeId`]
    /// matches [`Type::type_id`]).
    pub fn reflect_any_mut<'a>(&self, value: &'a mut dyn Any) -> Option<&'a mut dyn Reflect> {
        if (*value).type_id() != self.type_id {
            return None;
        }
        let ptr = NonNull::from(value).cast::<u8>();
        // SAFETY: `value` is a valid value of this type (checked above), uniquely borrowed for `'a`
        Some(unsafe { self.as_reflect_mut(PtrMut::new(ptr)) })
    }

    /// Casts the given `ptr`, whose value is of this [`Type`] to a [`Reflect`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `ptr` is a valid, initialized value that matches this [`Type`].
    pub unsafe fn as_reflect<'a>(&self, ptr: Ptr<'a>) -> &'a dyn Reflect {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `Type`, and `Type::new`
        // guarantees that the cast_reflect implementation is valid
        unsafe { &*self.as_reflect_ptr(ptr.as_ptr()) }
    }

    /// Casts the given `ptr`, whose value is of this [`Type`] to a mutable [`Reflect`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `ptr` is a valid, initialized value that matches this [`Type`].
    pub unsafe fn as_reflect_mut<'a>(&self, ptr: PtrMut<'a>) -> &'a mut dyn Reflect {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `Type`, and `Type::new`
        // guarantees that the cast_reflect implementation is valid
        unsafe { &mut *self.as_reflect_ptr(ptr.as_ptr()).cast_mut() }
    }
}

/// Returns whether `values` has exactly one value of each of the `types`, in order.
fn types_match(values: &[Box<dyn Reflect>], types: impl ExactSizeIterator<Item = TypeRef>) -> bool {
    values.len() == types.len()
        && values
            .iter()
            .zip(types)
            .all(|(value, ty)| value.ty().type_id() == ty.get().type_id())
}

/// # Safety
///
/// [`Typed::TYPE`] must accurately describe `Self` (see [`Type::new`]). Its
/// [`type_id`](Type::type_id) must be the [`TypeId`] of `Self`.
pub unsafe trait Typed: Any {
    /// The [`Type`] of this type.
    const TYPE: &'static Type;
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Type")
            .field("type_id", &self.type_id)
            .field("kind", &self.kind)
            .field("layout", &self.layout)
            .field("type_path", &self.type_path)
            .finish()
    }
}

/// Type-erased functions for a [`Type`].
///
/// Every function that takes a [`Ptr`] requires that the pointer is a valid, initialized value of
/// the described type.
///
/// Optional functions are `None` if the type does not support the operation. Operations like
/// [`Clone`], [`Debug`], and [`PartialEq`] fall back to structural implementations driven by [`Kind`] when they
/// are `None` (see [`Reflect`](crate::Reflect)).
pub struct TypeVTable {
    /// Casts the pointer to a dyn Reflect pointer using this type's Reflect vtable.
    pub cast_reflect: fn(*const u8) -> *const dyn Reflect,
    /// Sets the given pointer to the "default" value for this type.
    pub set_default: Option<unsafe fn(*mut u8)>,
    /// Returns a clone of the value, or `None` if any part of the value cannot be cloned.
    pub clone: Option<unsafe fn(Ptr<'_>) -> Option<Box<dyn Reflect>>>,
    /// Writes the debug formatting of a pointer to this type, to the given formatter.
    pub debug: Option<unsafe fn(Ptr<'_>, &mut Formatter<'_>) -> fmt::Result>,
    /// Hashes a pointer of this type using the configured hashing logic.
    pub hash: Option<unsafe fn(Ptr<'_>, &mut dyn Hasher)>,
    /// Returns true if the two pointers of the described type are equal.
    pub partial_eq: Option<unsafe fn(Ptr<'_>, Ptr<'_>) -> bool>,
    /// Returns an ordering if the two pointers of the described type have an ordering.
    pub partial_cmp: Option<unsafe fn(Ptr<'_>, Ptr<'_>) -> Option<Ordering>>,
}

impl TypeVTable {
    /// Creates a vtable for `T` with every optional function set to `None`.
    pub const fn new<T: Reflect>() -> Self {
        Self {
            cast_reflect: cast_reflect::<T>,
            set_default: None,
            clone: None,
            debug: None,
            hash: None,
            partial_eq: None,
            partial_cmp: None,
        }
    }
}

/// Casts the pointer to a dyn Reflect pointer using this type's Reflect vtable.
pub fn cast_reflect<T: Reflect>(ptr: *const u8) -> *const dyn Reflect {
    ptr.cast::<T>() as *const dyn Reflect
}

/// Implements [`TypeVTable::clone`] using `T`'s [`Clone`] implementation.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
pub unsafe fn clone<T: Clone + Reflect>(ptr: Ptr<'_>) -> Option<Box<dyn Reflect>> {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    Some(Box::new(unsafe { ptr.deref::<T>() }.clone()))
}

/// Implements [`TypeVTable::clone`] using `func`. Used for custom clone functions.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
#[inline]
pub unsafe fn clone_with<T: Reflect>(
    ptr: Ptr<'_>,
    func: impl FnOnce(&T) -> Option<T>,
) -> Option<Box<dyn Reflect>> {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    let value = func(unsafe { ptr.deref::<T>() })?;
    Some(Box::new(value))
}

/// Implements [`IgnoredFieldVTable::clone`](crate::IgnoredFieldVTable::clone) using `T`'s
/// [`Clone`] implementation.
///
/// # Safety
///
/// `src` must be a valid, initialized `T`, and `dst` must be a valid, unique allocation for a `T`.
/// Any existing value at `dst` will be overwritten without being dropped.
pub unsafe fn clone_into<T: Clone>(src: Ptr<'_>, dst: *mut u8) {
    // SAFETY: the caller guarantees that `src` is a valid `T`, and that `dst` is a valid, unique
    // allocation for a `T`
    unsafe { dst.cast::<T>().write(src.deref::<T>().clone()) }
}

/// Implements [`IgnoredFieldVTable::clone`](crate::IgnoredFieldVTable::clone) using `func`.
///
/// # Safety
///
/// `src` must be a valid, initialized `T`, and `dst` must be a valid, unique allocation for a `T`.
/// Any existing value at `dst` will be overwritten without being dropped.
#[inline]
pub unsafe fn clone_into_with<T>(src: Ptr<'_>, dst: *mut u8, func: impl FnOnce(&T) -> T) {
    // SAFETY: the caller guarantees that `src` is a valid `T`, and that `dst` is a valid, unique
    // allocation for a `T`
    unsafe { dst.cast::<T>().write(func(src.deref::<T>())) }
}

/// Implements [`TypeVTable::debug`] using `T`'s [`Debug`] implementation.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
pub unsafe fn debug<T: Debug>(ptr: Ptr<'_>, f: &mut Formatter<'_>) -> fmt::Result {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    Debug::fmt(unsafe { ptr.deref::<T>() }, f)
}

/// Implements [`TypeVTable::debug`] using `func`.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
#[inline]
pub unsafe fn debug_with<T>(
    ptr: Ptr<'_>,
    f: &mut Formatter<'_>,
    func: impl FnOnce(&T, &mut Formatter<'_>) -> fmt::Result,
) -> fmt::Result {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    func(unsafe { ptr.deref::<T>() }, f)
}

/// Implements [`TypeVTable::hash`] using `T`'s [`Hash`] implementation.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
pub unsafe fn hash<T: Hash>(ptr: Ptr<'_>, mut hasher: &mut dyn Hasher) {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    Hash::hash(unsafe { ptr.deref::<T>() }, &mut hasher);
}

/// Implements [`TypeVTable::hash`] using `func`, which returns the hash of the value.
///
/// # Safety
///
/// `ptr` must be a valid, initialized `T`.
#[inline]
pub unsafe fn hash_with<T>(ptr: Ptr<'_>, hasher: &mut dyn Hasher, func: impl FnOnce(&T) -> u64) {
    // SAFETY: the caller guarantees that `ptr` is a valid `T`
    hasher.write_u64(func(unsafe { ptr.deref::<T>() }));
}

/// Implements [`TypeVTable::partial_eq`] using `T`'s [`PartialEq`] implementation.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
pub unsafe fn partial_eq<T: PartialEq>(a: Ptr<'_>, b: Ptr<'_>) -> bool {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { a.deref::<T>() == b.deref::<T>() }
}

/// Implements [`TypeVTable::partial_eq`] using `func`.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
#[inline]
pub unsafe fn partial_eq_with<T>(
    a: Ptr<'_>,
    b: Ptr<'_>,
    func: impl FnOnce(&T, &T) -> bool,
) -> bool {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { func(a.deref::<T>(), b.deref::<T>()) }
}

/// Implements [`TypeVTable::partial_cmp`] using `T`'s [`PartialOrd`] implementation.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
pub unsafe fn partial_cmp<T: PartialOrd>(a: Ptr<'_>, b: Ptr<'_>) -> Option<Ordering> {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { a.deref::<T>().partial_cmp(b.deref::<T>()) }
}

/// Implements [`TypeVTable::partial_cmp`] using `func`.
///
/// # Safety
///
/// `a` and `b` must be valid, initialized `T` values.
#[inline]
pub unsafe fn partial_cmp_with<T>(
    a: Ptr<'_>,
    b: Ptr<'_>,
    func: impl FnOnce(&T, &T) -> Option<Ordering>,
) -> Option<Ordering> {
    // SAFETY: the caller guarantees that `a` and `b` are valid `T` values
    unsafe { func(a.deref::<T>(), b.deref::<T>()) }
}

/// Sets the given pointer to the "default" value for this type.
///
/// # Safety
///
/// `ptr` must be a valid, unique allocation for a `T`. Any existing value will be overwritten without
/// being dropped.
pub unsafe fn set_default<T: Default>(ptr: *mut u8) {
    // SAFETY: the caller guarantees that `ptr` is a valid, unique allocation for a `T`
    unsafe { set_with(ptr, T::default) }
}

/// Writes the result of `func` to `ptr`. Used to implement [`TypeVTable::set_default`] for custom
/// default functions.
///
/// # Safety
///
/// `ptr` must be a valid, unique allocation for a `T`. Any existing value will be overwritten without
/// being dropped.
#[inline]
pub unsafe fn set_with<T>(ptr: *mut u8, func: impl FnOnce() -> T) {
    // SAFETY: the caller guarantees that `ptr` is a valid, unique allocation for a `T`. `write` does
    // not drop the (uninitialized) previous value.
    unsafe { ptr.cast::<T>().write(func()) }
}
