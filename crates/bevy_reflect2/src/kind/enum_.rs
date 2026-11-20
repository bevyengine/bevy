use core::{hint::unreachable_unchecked, mem::MaybeUninit, ptr::NonNull};

use bevy_ptr::{Ptr, PtrMut};

use crate::{
    kind::{FieldKind, IgnoredFieldVTable},
    metadata::{impl_metadata_methods, Metadata},
    Reflect, TypeRef,
};

/// Describes the variants of an enum.
///
/// Unsafe code relies on `cast_enum_access` being valid for the described type, so this can only be
/// constructed using [`Enum::new`].
//
// TODO: Implement offset-driven fast-path for repr C types
//
// # Rust Feature Wishlist
//  - [offset_of_enum](https://github.com/rust-lang/rust/issues/120141): Would enable efficient
//    offset-driven field access and reusing the Field / StructRef types
#[derive(Debug)]
pub struct Enum {
    variants: &'static [Variant],
    cast_enum_access: fn(*const u8) -> *const dyn EnumAccess,
}

impl Enum {
    /// Create a new enum definition.
    ///
    /// # Safety
    ///
    /// `variants` must describe every variant of the enum, in declaration order, and
    /// `cast_enum_access` must cast pointers to the enum to its [`EnumAccess`] implementation (see
    /// [`cast_enum_access`]). `construct` must only return values of the enum, with the requested
    /// variant (see [`InitializeEnumVariantFn`]).
    ///
    /// Together, [`Variant::fields`] and [`Variant::ignored_fields`] must contain every field of that
    /// variant exactly once, each with the correct type (see [`IgnoredFieldVTable`] for the
    /// requirements of ignored fields).
    pub const unsafe fn new(
        variants: &'static [Variant],
        cast_enum_access: fn(*const u8) -> *const dyn EnumAccess,
    ) -> Self {
        Self {
            variants,
            cast_enum_access,
        }
    }

    /// The variants of this enum, in declaration order.
    #[inline]
    pub const fn variants(&self) -> &'static [Variant] {
        self.variants
    }

    /// Returns the index of the variant with `name` (in [`Enum::variants`]), or `None` if there is
    /// no such variant.
    pub fn variant_index(&self, name: &str) -> Option<usize> {
        self.variants
            .iter()
            .position(|variant| variant.name == name)
    }

    /// Casts the given `enum_ptr` of this type to an [`EnumAccess`] reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `enum_ptr` is a valid, initialized value that matches this [`Enum`].
    pub unsafe fn enum_access<'a>(&self, enum_ptr: Ptr<'a>) -> &'a dyn EnumAccess {
        // SAFETY: Caller guarantees that ptr matches the type defined in this `Enum`, and `Enum::new`
        // guarantees that the cast_enum_access implementation is valid
        unsafe { &*(self.cast_enum_access)(enum_ptr.as_ptr()) }
    }
}

/// Describes a variant of an enum.
#[derive(Debug)]
pub struct Variant {
    /// The name of the variant. (ex: for `Option::Some`, this would be "Some").
    pub name: &'static str,
    /// The [`FieldKind`] of the variant.
    pub field_kind: FieldKind,
    /// The fields of the [`Variant`] in declaration order.
    pub fields: &'static [EnumField],
    /// The fields that are not reflected (ex: fields marked `#[reflect(ignore)]`), which are missing
    /// from [`Variant::fields`].
    pub ignored_fields: &'static [EnumIgnoredField],
    /// The metadata of the variant (registered with `#[reflect(@expr)]`).
    pub metadata: Metadata,
    /// Initializes a value of this variant from its fields.
    pub init: Option<InitializeEnumVariantFn>,
}

impl Variant {
    impl_metadata_methods!(self, self.metadata, "variant");

    /// Returns the index of the reflected field with `name` (in [`Variant::fields`]), or `None`
    /// if there is no such field. Unnamed fields are named by their index (ex: `"0"`), and
    /// [ignored fields](Variant::ignored_fields) are not included.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|field| field.name == name)
    }

    /// Initializes a new value of this variant into `dst`, initializing each field of the variant
    /// with `func`.
    ///
    /// `func` is called exactly once for each field in the variant, in declaration order. `func`
    /// must return whether it initialized the passed-in pointer. If `func` returns `false`, the
    /// initialization bails out and won't call `func` again.
    ///
    /// The value in `dst` is overwritten and **will not** be dropped. Ignored fields are
    /// initialized with the field's default implementation.
    ///
    /// If this function returns `true`, `dst` holds a fully initialized value of this variant.
    ///
    /// # Safety
    ///
    /// 1) This variant must have an `init` implementation.
    /// 2) `dst` must be valid for writes of this enum type.
    /// 3) `func` must return whether it initialized the passed-in pointer. It must initialize to a
    ///    valid value for the field at the provided index in the variant. `func` may not read/drop
    ///    the previous value (as the pointer may point to an uninitialized or overwritten
    ///    location).
    // Note: This function doesn't really do anything different from calling `self.init` directly,
    // it's just slightly more convenient to call.
    pub unsafe fn initialize_variant_raw(
        &self,
        mut func: impl FnMut(usize, NonNull<u8>) -> bool,
        dst: NonNull<u8>,
    ) -> bool {
        let Some(init) = self.init else {
            // SAFETY: By the safety precondition, we know there is an `init` implementation.
            unsafe { unreachable_unchecked() };
        };
        // SAFETY: By the safety precondition, `dst` is valid for writes of this enum type, and
        // `func` will initialize all the field types to a valid value.
        unsafe { (init)(&mut func, dst) }
    }
}

/// Describes the field of an [`Enum`] [`Variant`].
#[derive(Debug)]
pub struct EnumField {
    /// The name of the field. For named fields, this will be "field". For "unnamed" fields, this will
    /// be the index in string form (ex: "0").
    pub name: &'static str,
    /// A reference to the [`Type`](crate::Type) of the field.
    pub ty: TypeRef,
    /// The metadata of the field (registered with `#[reflect(@expr)]`).
    pub metadata: Metadata,
}

impl EnumField {
    impl_metadata_methods!(self, self.metadata, "field");
}

/// Describes the field of an [`Enum`] [`Variant`] that is not reflected (ex: a field marked `#[reflect(ignore)]`). Its
/// offset depends on the variant's layout, so it is found using [`EnumRef::ignored_field_offset`].
#[derive(Debug)]
pub struct EnumIgnoredField {
    /// The name of the field. For named fields, this will be "field". For "unnamed" fields, this will
    /// be the index in string form (ex: "0").
    pub name: &'static str,
    /// The vtable of the ignored field.
    pub vtable: IgnoredFieldVTable,
}

/// # Safety
///
/// [`EnumAccess::variant`] must return the index of the active variant (matching [`Enum::variants`]),
/// and [`EnumAccess::field_offset`] must return the offset (from the start of the enum) of the field
/// at the given index of that variant, or `None` if the index is out of bounds. Unsafe code accesses
/// fields at these offsets, using the types described in [`Variant::fields`] and
/// [`Variant::ignored_fields`].
pub unsafe trait EnumAccess: 'static {
    /// Returns the index of the current variant.
    fn variant(&self) -> usize;
    /// Returns the offset in bytes (from the start of the enum) of the field at `index` of the
    /// active variant, or `None` if it doesn't exist. Indices `0..fields.len()` are the
    /// [reflected fields](Variant::fields), and the following indices are the
    /// [ignored fields](Variant::ignored_fields).
    fn field_offset(&self, index: usize) -> Option<usize>;
}

/// Casts the given pointer to an [`EnumAccess`] pointer.
pub fn cast_enum_access<E: EnumAccess>(ptr: *const u8) -> *const dyn EnumAccess {
    ptr.cast::<E>() as *const dyn EnumAccess
}

/// A reflected reference to an [`Enum`] value, providing read-only reflection operations on that value.
#[derive(Clone, Copy)]
pub struct EnumRef<'a> {
    // SAFETY NOTE: this should only ever be constructed with EnumRef::new
    ptr: Ptr<'a>,
    // SAFETY NOTE: this should only ever be constructed with EnumRef::new
    enum_type: &'static Enum,
}

impl<'a> EnumRef<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `enum_type`
    pub unsafe fn new(ptr: Ptr<'a>, enum_type: &'static Enum) -> Self {
        Self { ptr, enum_type }
    }

    /// The [`Enum`] of this type.
    #[inline]
    pub fn enum_type(&self) -> &'static Enum {
        self.enum_type
    }

    #[inline]
    fn access(&self) -> &'a dyn EnumAccess {
        // SAFETY: `EnumRef::new` guarantees that `ptr` matches `enum_type`
        unsafe { self.enum_type.enum_access(self.ptr) }
    }

    /// Accesses the field at the given `index`. Returns [`None`] if the index is out of bounds.
    /// The field is accessed for the current variant.
    pub fn field(&self, index: usize) -> Option<&'a dyn Reflect> {
        let access = self.access();
        let field = self.enum_type.variants[access.variant()]
            .fields
            .get(index)?;
        let offset = access.field_offset(index)?;
        // SAFETY: `EnumAccess` guarantees that `offset` is the offset of the reflected field at
        // `index` of the active variant, and `Enum::new` guarantees that `field.ty` is its type
        Some(unsafe { field.ty.get().as_reflect(self.ptr.byte_add(offset)) })
    }

    /// Accesses the field with the given `name`. Returns [`None`] if no field with that name exists.
    /// The field is accessed for the current variant.
    pub fn field_by_name(&self, name: &str) -> Option<&'a dyn Reflect> {
        self.field(self.variant_type().field_index(name)?)
    }

    /// Returns the offset in bytes of the [reflected field](Variant::fields) at `index` of the
    /// active variant, or `None` if it doesn't exist.
    #[inline]
    pub fn field_offset(&self, index: usize) -> Option<usize> {
        if index >= self.variant_type().fields.len() {
            return None;
        }
        self.access().field_offset(index)
    }

    /// Returns the offset in bytes of the [ignored field](Variant::ignored_fields) at `index` of the
    /// active variant, or `None` if it doesn't exist.
    #[inline]
    pub fn ignored_field_offset(&self, index: usize) -> Option<usize> {
        let variant = self.variant_type();
        if index >= variant.ignored_fields.len() {
            return None;
        }
        self.access().field_offset(variant.fields.len() + index)
    }

    /// Returns the index of the current variant.
    #[inline]
    pub fn variant(&self) -> usize {
        self.access().variant()
    }

    /// Returns the [`Variant`] description of the active variant.
    #[inline]
    pub fn variant_type(&self) -> &'static Variant {
        &self.enum_type.variants[self.variant()]
    }

    /// Iterates over each field description and value of the active variant, in declaration order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (&'static EnumField, &'a dyn Reflect)> + Clone + use<'a>
    {
        let access = self.access();
        let ptr = self.ptr;
        self.enum_type.variants[access.variant()]
            .fields
            .iter()
            .enumerate()
            .map(move |(index, field)| {
                let offset = access
                    .field_offset(index)
                    .expect("index is a valid field of the variant");
                // SAFETY: `EnumAccess` guarantees that `offset` is the offset of the reflected field
                // at `index` of the active variant, and `Enum::new` guarantees that `field.ty` is
                // its type
                (field, unsafe {
                    field.ty.get().as_reflect(ptr.byte_add(offset))
                })
            })
    }
}

/// A reflected mutable reference to an [`Enum`] value, providing reflection operations on that value.
pub struct EnumMut<'a> {
    // SAFETY NOTE: this should only ever be constructed with EnumMut::new
    ptr: PtrMut<'a>,
    // SAFETY NOTE: this should only ever be constructed with EnumMut::new
    enum_type: &'static Enum,
}

impl<'a> EnumMut<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `enum_type`
    pub unsafe fn new(ptr: PtrMut<'a>, enum_type: &'static Enum) -> Self {
        Self { ptr, enum_type }
    }

    /// Returns a read-only view of this enum.
    #[inline]
    pub fn as_ref(&self) -> EnumRef<'_> {
        EnumRef {
            ptr: self.ptr.as_ref(),
            enum_type: self.enum_type,
        }
    }

    /// The [`Enum`] of this type.
    #[inline]
    pub fn enum_type(&self) -> &'static Enum {
        self.enum_type
    }

    /// Accesses the field at the given `index`. Returns [`None`] if the index is out of bounds.
    /// The field is accessed for the current variant.
    #[inline]
    pub fn field(&self, index: usize) -> Option<&dyn Reflect> {
        self.as_ref().field(index)
    }

    /// Mutably accesses the field at the given `index`. Returns [`None`] if the index is out of bounds.
    /// The field is accessed for the current variant.
    pub fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        // The shared borrow used to find the field ends before the field is mutably borrowed
        let field = self.variant_type().fields.get(index)?;
        let offset = self.as_ref().field_offset(index)?;
        // SAFETY: `EnumAccess` guarantees that `offset` is the offset of the reflected field at
        // `index` of the active variant, and `Enum::new` guarantees that `field.ty` is its type. The
        // returned borrow is tied to `&mut self`, which prevents aliasing.
        Some(unsafe {
            field
                .ty
                .get()
                .as_reflect_mut(self.ptr.reborrow().byte_add(offset))
        })
    }

    /// Accesses the field with the given `name`. Returns [`None`] if no field with that name exists.
    /// The field is accessed for the current variant.
    #[inline]
    pub fn field_by_name(&self, name: &str) -> Option<&dyn Reflect> {
        self.as_ref().field_by_name(name)
    }

    /// Mutably accesses the field with the given `name`. Returns [`None`] if no field with that name exists.
    /// The field is accessed for the current variant.
    pub fn field_by_name_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        self.field_mut(self.variant_type().field_index(name)?)
    }

    /// Returns the index of the current variant.
    #[inline]
    pub fn variant(&self) -> usize {
        self.as_ref().variant()
    }

    /// Returns the [`Variant`] description of the active variant.
    #[inline]
    pub fn variant_type(&self) -> &'static Variant {
        self.as_ref().variant_type()
    }
}

/// Creates an owned value of type `T` by initializing the value using `func`.
///
/// # Safety
///
/// 1) `field_index` must correspond to the index of a field of type `T` in an enum variant that
///    `func` expects.
/// 2) `func` must satisfy the same safety requirements as [`InitializeEnumVariantFn`]'s func
///    argument.
pub unsafe fn create_enum_field_value_from_init<T>(
    field_index: usize,
    func: &mut dyn FnMut(usize, NonNull<u8>) -> bool,
) -> Option<T> {
    let mut value = MaybeUninit::<T>::uninit();
    // SAFETY: `value` is a stack variable, so its pointer cannot be null.
    let value_ptr = unsafe { NonNull::new_unchecked(value.as_mut_ptr().cast::<u8>()) };
    if !func(field_index, value_ptr) {
        // The value wasn't initialized, so there's nothing more to do. We don't need to drop
        // anything either.
        return None;
    }
    // SAFETY: By the safety precondition, `func` returns true if and only if it initialized the
    // `value_ptr` to a valid value of the enum's field.
    Some(unsafe { value.assume_init() })
}

/// Initializes the variant of an enum from its fields.
///
/// We will assume the names of the arguments are `func` and `dst`. `func` is called with the index
/// of each reflected field of the variant (in [`Variant::fields`]), in declaration order. `func`
/// must return whether it initialized the passed-in pointer. If `func` returns `false`, the
/// initialization bails out and won't call `func` again.
///
/// The value in `dst` is overwritten and **will not** be dropped. Ignored fields are initialized
/// with their default.
///
/// If this function returns `true`, `dst` holds a fully-initialized value of this variant.
///
/// # Safety
///
/// 1) `dst` must be valid for writes of this enum type.
/// 2) `func` must return whether it initialized the passed-in pointer. It must initialize to a
///    valid value for the field at the provided index in the variant. `func` may not read/drop the
///    previous value (as the pointer may point to an uninitialized or overwritten location).
pub type InitializeEnumVariantFn =
    unsafe fn(&mut dyn FnMut(usize, NonNull<u8>) -> bool, NonNull<u8>) -> bool;
