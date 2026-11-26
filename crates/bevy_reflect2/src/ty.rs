use bevy_ptr::{Ptr, PtrMut};

use crate::{
    builtin::move_box_into,
    kind::{Kind, Struct, StructField, UncoveredBytes},
    metadata::impl_metadata_methods,
    metadata::Metadata,
    reflect::Reflect,
    type_path::TypePath,
    type_vtable::TypeVTable,
};
use alloc::{
    alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error},
    boxed::Box,
};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    cell::Cell,
    fmt::Debug,
    hint::unreachable_unchecked,
    mem::DropGuard,
    ptr::{self, NonNull},
};

/// Implemented for Rust types that have an associated [`Type`].
///
/// # Safety
///
/// [`Typed::TYPE`] must accurately describe `Self` (see [`Type::new`]). Its
/// [`type_id`](Type::type_id) must be the [`TypeId`] of `Self`.
pub unsafe trait Typed: Any {
    /// The [`Type`] of this type.
    const TYPE: &'static Type;
}

/// Describes a Rust type.
///
/// Unsafe code relies on this information being accurate (ex: field offsets, layout, and vtable
/// functions), so it can only be constructed using [`Type::new`].
#[derive(Debug)]
pub struct Type {
    type_id: TypeId,
    kind: Kind,
    layout: Option<Layout>,
    vtable: TypeVTable,
    type_path: TypePath,
    metadata: Metadata,
}

impl Type {
    /// Creates a new [`Type`] description.
    ///
    /// # Safety
    ///
    /// Every argument must accurately describe the type identified by `type_id`:
    /// - `kind` must match the type's shape (ex: [`Struct`] field offsets and types).
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
            metadata: Metadata::EMPTY,
        }
    }

    /// Sets the [`Metadata`] of this type (registered with `#[reflect(@expr)]`).
    #[inline]
    pub const fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
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

    /// Returns the [`Metadata`] of this type (registered with `#[reflect(@expr)]`).
    #[inline]
    pub const fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    impl_metadata_methods!(self, self.metadata, "type");

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

    /// Initializes a new value of this struct type into `dst`, initializing each field of the
    /// struct with `func`.
    ///
    /// `func` is called exactly once for each field in the struct, and in the same order as
    /// [`Struct::fields`]. `func` must return whether it initialized the passed-in pointer. If
    /// `func` returns `false`, the initialization bails out and won't call `func` again.
    ///
    /// The value in `dst` is overwritten and **will not** be dropped. Ignored fields are
    /// initialized with the field's default implementation. If an ignored field's default
    /// implementation panics, all previously written ignored fields are leaked.
    ///
    /// If this function returns `true`, `dst` holds a fully-initialized value of this type.
    ///
    /// Consider using [`Self::initialize_struct`] or [`Self::construct_struct`] for versions that
    /// perform the allocation of the destination for you (and have fewer safety requirements).
    ///
    /// # Safety
    ///
    /// 1) This type must be a [`Kind::Struct`].
    /// 2) This type's [`Self::layout`] must be [`Some`] (i.e., the type must be sized).
    /// 3) `dst` must be valid for writes of this type.
    /// 4) `func` must return whether it initialized the passed-in pointer. It must initialize to a
    ///    valid value that matches the type described in each [`StructField`]. `func` may not
    ///    read/drop the previous value (as the pointer may point to an uninitialized or overwritten
    ///    location).
    /// 5) All ignored fields of the struct must have a reflected default impl (or there are no
    ///    ignored fields).
    /// 6) Either `uncovered_initialized` is `false`, **or** the uncovered bytes in `dst` must be
    ///    correctly initialized according to [`Struct::uncovered_bytes`].
    pub unsafe fn initialize_struct_raw(
        &self,
        dst: NonNull<u8>,
        uncovered_initialized: bool,
        mut func: impl FnMut(&StructField, NonNull<u8>) -> bool,
    ) -> bool {
        let Kind::Struct(struct_type) = &self.kind else {
            // SAFETY: The safety precondition guarantees the type is a struct.
            unsafe { unreachable_unchecked() };
        };
        // This check allows us to avoid writing zeroed bytes over top an allocation that was
        // allocated with `alloc_zeroed` (which can be more efficient, then `alloc` + `memset`).
        if !uncovered_initialized {
            match struct_type.uncovered_bytes() {
                // Do nothing. Uncovered bytes can be uninitialized.
                UncoveredBytes::Padding => {}
                UncoveredBytes::Zeroed => {
                    // SAFETY: The safety precondition guarantees that the type is sized (and therefore
                    // that the layout is `Some`).
                    let layout = unsafe { self.layout.unwrap_unchecked() };
                    // SAFETY: By the safety precondition, `dst` is valid for writes of this type, and
                    // we know the type has the specified layout by the safety of `Type::new`, so the
                    // size is correct.
                    unsafe { dst.as_ptr().write_bytes(0, layout.size()) };
                }
            }
        }
        let initialized_fields = Cell::new(0);
        let drop_set_fields_guard = DropGuard::new((), |_| {
            // Iterate in reverse order so that the most recent values are dropped first (to mimic
            // how stack variables are dropped).
            for field in struct_type.fields()[0..initialized_fields.get()]
                .iter()
                .rev()
            {
                // SAFETY: By the safety precondition, `dst` is valid for writes of this type, and
                // `Type::new` guarantees that `field.offset` is in bounds of it.
                let field_ptr = unsafe { dst.byte_add(field.offset) }.as_ptr();
                let reflect_ptr = field.ty.get().as_reflect_ptr(field_ptr);
                // SAFETY: By `Type::new`, we know that `field.offset` is correct. By the safety
                // precondition, we know `dst` is valid for writes, so therefore `dst+field.offset`
                // must be valid for writes of the field type. We casted the field pointer to the
                // field type, and we know we currently own the value for `dst`.
                unsafe {
                    ptr::drop_in_place(reflect_ptr.cast_mut());
                }
            }
        });
        for field in struct_type.fields() {
            // SAFETY: By the safety precondition, `dst` is valid for writes of this type, and
            // `Type::new` guarantees that `field.offset` is in bounds of it.
            let field_ptr = unsafe { dst.byte_add(field.offset) };
            if !(func)(field, field_ptr) {
                // The function returned false, meaning it didn't initialize the field_ptr. So bail
                // out and let the drop guard drop the previous initialized fields.
                return false;
            }
            // This field is initialized, so keep track of it so we properly drop the field if we
            // panic or bail out later.
            initialized_fields.update(|x| x + 1);
        }
        // SAFETY: By the safety precondition, `dst` is valid for writes of the `struct_type` and
        // all ignored fields have a reflected default impl.
        unsafe { initialize_ignored_fields(struct_type, dst) };

        // At this point, all the uncovered bytes are initialized (either by the caller if
        // `uncovered_initialized` is true, or by us otherwise), all the normal fields have been
        // initialized with func, and all the ignored fields have been defaulted (and by
        // `Struct::new` these are all the fields in the struct). Therefore, `dst` is fully
        // initialized, and we don't need to drop the fields on anymore.
        DropGuard::dismiss(drop_set_fields_guard);
        true
    }

    /// Allocates a new value of this type, initializing each field of the struct with `func`.
    ///
    /// Bytes that are not covered by a field are left uninitialized if they are padding, or zeroed
    /// before `func` is called if the struct requires it (see [`UncoveredBytes`]).
    ///
    /// [Ignored fields](crate::kind::Struct::ignored_fields) are initialized with their
    /// [`default`](crate::kind::IgnoredFieldVTable::default) function (registered with
    /// `#[reflect(default)]`) after `func` initializes the reflected fields.
    ///
    /// Returns `None` if this type is not a [`Kind::Struct`], is unsized, or has an ignored field
    /// without a `default` function.
    ///
    /// If an ignored field default implementation panics, all previously written ignored fields are
    /// leaked.
    ///
    /// This is like [`Self::initialize_struct_raw`] but with 1) fewer safety requirements, and 2)
    /// allocates a box to store the type for you.
    ///
    /// # Safety
    ///
    /// `func` must return whether it initialized the passed-in pointer. It must initialize to a
    /// valid value that matches the type describe in each [`StructField`]. `func` may not read/drop
    /// the previous value (as the pointer may point to an uninitialized location).
    pub unsafe fn initialize_struct(
        &self,
        func: impl FnMut(&StructField, NonNull<u8>) -> bool,
    ) -> Option<Box<dyn Reflect>> {
        let Kind::Struct(struct_type) = &self.kind else {
            return None;
        };
        let ptr = self.alloc_struct(struct_type)?;
        let dealloc_guard = DropGuard::new((), |_| {
            // SAFETY: `ptr` was allocated with `Type::alloc` (called by `alloc_struct`), and we
            // only drop this pointer if we failed initialization (in which case we don't use the
            // pointer).
            unsafe { self.dealloc(ptr) };
        });
        // SAFETY:
        // 1) We've checked the type is `Kind::Struct`.
        // 2) `alloc_struct` returns `None` if the type is unsized, so we know the type is sized.
        // 3) We just allocated `ptr`, and with the layout of this type, so it is valid for writes.
        // 4) By the safety precondition, `func` returns whether it initialized the passed-in
        //    pointer, and initializes to a valid value for the field type.
        // 5) `alloc_struct` returns `None` if any ignored fields don't have a default impl.
        // 6) `alloc_struct` allocates the pointer based on the struct's `Struct::uncovered_bytes`.
        let initialized = unsafe {
            self.initialize_struct_raw(ptr, /*uncovered_initialized=*/ true, func)
        };
        if !initialized {
            return None;
        }
        DropGuard::dismiss(dealloc_guard);

        // SAFETY: `initialize_struct_raw` guarantees that the type is fully initialized.
        // `alloc_struct` also allocated `ptr` with the global allocator (or the type is a ZST).
        Some(unsafe { self.box_from_raw(ptr) })
    }

    /// Allocates memory for a value of `struct_type` (the [`Kind`] of this type), which is zeroed
    /// if the struct requires it (see [`UncoveredBytes`]).
    ///
    /// Returns `None` if this type is unsized, or has an ignored field without a `default`
    /// function (which [`initialize_ignored_fields`] could not initialize).
    fn alloc_struct(&self, struct_type: &Struct) -> Option<NonNull<u8>> {
        if struct_type
            .ignored_fields()
            .iter()
            .any(|field| field.vtable.default.is_none())
        {
            return None;
        }
        match struct_type.uncovered_bytes() {
            UncoveredBytes::Padding => self.alloc(),
            UncoveredBytes::Zeroed => self.alloc_zeroed(),
        }
    }

    /// Constructs a new value of this struct type from the values of its reflected fields, in the
    /// order of [`Struct::fields`](crate::kind::Struct::fields). [Ignored
    /// fields](crate::kind::Struct::ignored_fields) are initialized with their default (see
    /// [`Type::initialize_struct`]).
    ///
    /// Returns `None` if this type is not a [`Kind::Struct`], the number of values doesn't match
    /// the number of fields, a value has the wrong type, or an ignored field has no default.
    ///
    /// This is a type-checked and safe version of [`Self::initialize_struct_raw`].
    pub fn construct_struct(
        &self,
        values: impl IntoIterator<Item = Box<dyn Reflect>>,
    ) -> Option<Box<dyn Reflect>> {
        let Kind::Struct(struct_type) = &self.kind else {
            return None;
        };
        let ptr = self.alloc_struct(struct_type)?;
        let ptr_dealloc = DropGuard::new((), |_| {
            // SAFETY: We allocated the ptr with [`Type::alloc`], and we only continue to use the
            // pointer if we successfully initialized the value (where this DropGuard is dismissed).
            unsafe { self.dealloc(ptr) };
        });

        let mut values = values.into_iter();
        // SAFETY:
        // 1) We've checked the type is a struct.
        // 2) `alloc_struct` fails if the layout is `None`, therefore layout must be `Some`.
        // 3) `ptr` was allocated with this struct's layout, and nothing else reads/writes to this
        //    pointer until after this call. So `ptr` is valid for writes of this type.
        // 4) The function only returns true if it initializes the pointer (by moving in the value).
        //    We check the type of the value to make sure it matches the field before moving.
        // 5) `alloc_struct` returns `None` if any ignored field does not have a reflected default
        //    impl. Therefore, all fields must have reflected default impls.
        // 6) `alloc_struct` guarantees to correctly initialize the bytes according to
        //    `Struct::uncovered_bytes`.
        let initialized = unsafe {
            self.initialize_struct_raw(
                ptr,
                /*uncovered_initialized=*/ true,
                |field, field_ptr| {
                    let Some(value) = values.next() else {
                        return false;
                    };
                    if value.as_ref().type_id() != field.ty.get().type_id() {
                        return false;
                    }
                    move_box_into(value, field_ptr.as_ptr());
                    true
                },
            )
        };

        if !initialized {
            // We failed to initialize. Just let the drop guard deallocate the memory.
            return None;
        }
        DropGuard::dismiss(ptr_dealloc);

        // SAFETY: We've initialized the regular fields and the ignored fields, and `Struct::new`
        // ensures this covers all fields, and `alloc_struct` ensures all uncovered bytes are
        // correctly set, so `ptr` is fully initialized. `alloc_struct` also allocated `ptr` with
        // the global allocator (or the type is a ZST).
        let result = unsafe { self.box_from_raw(ptr) };

        if values.next().is_some() {
            // If there are still fields, return None. The initialized value gets dropped by the
            // Box.
            return None;
        }

        Some(result)
    }

    /// Initializes a new value of this array type into `dst`, initializing each element of the
    /// array with `func`.
    ///
    /// `func` is called exactly once for each element in the array, and in the order of the
    /// elements.
    ///
    /// The value in `dst` is overwritten and **will not** be dropped.
    ///
    /// If this function returns `true`, `dst` holds a fully-initialized value of this type.
    ///
    /// # Safety
    ///
    /// 1) This type must be a [`Kind::Array`].
    /// 2) `dst` must be valid for writes of this type.
    /// 3) `func` must initialize every pointer to a valid value that matches the [`Array::item`] of
    ///    this array. `func` may not read/drop the previous valid (as the pointer may point to an
    ///    uninitialized or overwritten location).
    ///
    /// [`Array::item`]: crate::kind::Array::item
    pub unsafe fn initialize_array_raw(
        &self,
        dst: NonNull<u8>,
        mut func: impl FnMut(usize, &Type, NonNull<u8>) -> bool,
    ) -> bool {
        let Kind::Array(array_ty) = &self.kind else {
            // SAFETY: The safety precondition guarantees the type is an array.
            unsafe { unreachable_unchecked() }
        };
        let item_ty = array_ty.item().get();
        let stride = array_ty.stride();

        let initialized_items = Cell::new(0);
        let drop_set_fields_guard = DropGuard::new((), |_| {
            // Iterate in reverse order so that the most recent values are dropped first (to mimic
            // how stack variables are dropped).
            for i in (0..initialized_items.get()).rev() {
                // SAFETY: We know that `dst` is valid for writes of this type, and we know
                // `i < array_ty.len()`. By `Array::new`, we also know the stride is accurate.
                // Therefore, this pointer does not exceed the bounds of the array's memory. If the
                // item type is a ZST, `dst` always adds zero, which is still a valid pointer.
                let item_ptr = unsafe { dst.byte_add(i * stride) };
                let reflect_ptr = item_ty.as_reflect_ptr(item_ptr.as_ptr());
                // SAFETY: We have exclusive access to `dst`, and therefore, the array element. We
                // know the index is within the bounds of the array, and we know the value is
                // initialized due to the counter we kept track of.
                unsafe { ptr::drop_in_place(reflect_ptr.cast_mut()) };
            }
        });

        for i in 0..array_ty.len() {
            // SAFETY: We know that `dst` is valid for writes of this type, and we know
            // `i < array_ty.len()`. By `Array::new`, we also know the stride is accurate.
            // Therefore, this pointer does not exceed the bounds of the array's memory. If the item
            // type is a ZST, `dst` always adds zero, which is still a valid pointer.
            let item_ptr = unsafe { dst.byte_add(i * stride) };
            if !(func)(i, item_ty, item_ptr) {
                // `func` failed to initialize the element, so just return false and let the drop
                // guard drop all the previous fields.
                return false;
            }
            // The element was initialized, so keep track of it so we drop it later.
            initialized_items.update(|x| x + 1);
        }
        // At this point, we've called `func` on the memory locations of all the elements of the
        // array. By the safety precondition, `func` guarantees it will initialize the ptr. So we
        // must have a fully-initialized value of this array.
        DropGuard::dismiss(drop_set_fields_guard);
        true
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
        let dst = self.alloc()?;
        let dealloc_guard = DropGuard::new((), |_| {
            // SAFETY: `ptr` was allocated with `Type::alloc` (called by `alloc_struct`), and we
            // only drop this pointer if we failed initialization (in which case we don't use the
            // pointer).
            unsafe { self.dealloc(dst) };
        });

        let item_ty = array_type.item().get();
        let mut items = items.into_iter();

        // SAFETY:
        // 1) We've checked the type is an array.
        // 2) `dst` was allocated with this array's layout, and nothing else reads/writes to this
        //    pointer until after this call. So `ptr` is valid for writes of this type.
        // 3) The function only returns true if it initialzies the pointer (by moving in the value).
        //    We check the type of the value to make sure it matches the item type before moving.
        let initialized = unsafe {
            self.initialize_array_raw(dst, |_, _, item_ptr| {
                let Some(item) = items.next() else {
                    return false;
                };
                if item.as_ref().type_id() != item_ty.type_id {
                    return false;
                }
                move_box_into(item, item_ptr.as_ptr());
                true
            })
        };
        if !initialized {
            // We failed to initialize. Just let the drop guard deallocate the memory.
            return None;
        }
        DropGuard::dismiss(dealloc_guard);
        // SAFETY: `dst` was allocated with `Type::alloc`, and every item was initialized above.
        // Arrays have no bytes other than their items.
        let result = unsafe { self.box_from_raw(dst) };

        // If there are still items, return None. The initialized value gets dropped by the Box.
        if items.next().is_some() {
            return None;
        }

        Some(result)
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

/// A lazy reference to a [`Type`] via a function pointer. This makes evaluation lazy, which moves
/// const-eval to runtime, preventing exponential const-evaluation costs / boosting compile times. In
/// a compile time benchmark 30 levels deep, the [`TypeRef`] version compiled in 0.11 seconds and direct
/// &'static Type nesting compiled in 37 seconds. [`TypeRef`] also allows for reflecting recursive types.
#[derive(Clone, Copy)]
pub struct TypeRef(pub fn() -> &'static Type);

impl TypeRef {
    /// Get the referenced type by calling the function.
    #[inline]
    pub fn get(&self) -> &'static Type {
        (self.0)()
    }
}

impl Debug for TypeRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TypeRef")
            .field(&self.get().type_path().short())
            .finish()
    }
}

/// Initializes the [ignored fields](Struct::ignored_fields) of the struct at `ptr` with
/// their default.
///
/// Overwritten values are leaked. If the default impl panics, all previously written values are
/// leaked.
///
/// # Safety
///
/// `ptr` must be valid for writes of the `struct_type`, and all ignored fields of `struct_type`
/// must have a default impl reflected.
unsafe fn initialize_ignored_fields(struct_type: &Struct, ptr: NonNull<u8>) {
    // TODO: Ignored fields are leaked if any of their default impls panic. We should store a drop
    // impl for ignored fields.
    for field in struct_type.ignored_fields() {
        // SAFETY: Safety precondition ensures that all ignored fields of `struct_type` has a
        // reflected default impl.
        let default = unsafe { field.vtable.default.unwrap_unchecked() };
        // SAFETY: `Struct::new` guarantees that `field.offset` is the location of the field in
        // this allocation, and that `default` is valid for the field's type
        unsafe { default(ptr.as_ptr().add(field.offset)) };
    }
}
