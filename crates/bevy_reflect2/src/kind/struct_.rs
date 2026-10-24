use crate::{ty::Type, Reflect};
use bevy_ptr::{Ptr, PtrMut};
use core::{alloc::Layout, fmt::Debug};

/// Describes the fields of a struct.
///
/// Unsafe code relies on the field offsets and types being accurate, so this can only be constructed
/// using [`Struct::new`].
#[derive(Debug)]
pub struct Struct {
    fields: &'static [Field],
    field_kind: FieldKind,
    struct_kind: StructKind,
    ignored_fields: &'static [IgnoredField],
    uncovered_bytes: UncoveredBytes,
}

/// Describes the field of a [`Struct`].
#[derive(Debug)]
pub struct Field {
    /// The name of the field. For named fields, this will be "field". For "unnamed" fields, this will
    /// be the index in string form (ex: "0").
    pub name: &'static str,
    /// The offset (in bytes) of the field, relative to the pointer of the struct it belongs to.
    pub offset: usize,
    /// A reference to the [`Type`] of the field.
    pub ty: TypeRef,
}

/// A [`Struct`] field that is not reflected (ex: a field marked `#[reflect(ignore)]`).
#[derive(Debug)]
pub struct IgnoredField {
    /// The name of the field. For named fields, this will be "field". For "unnamed" fields, this will
    /// be the index in string form (ex: "0").
    pub name: &'static str,
    /// The offset (in bytes) of the field, relative to the pointer of the struct it belongs to.
    pub offset: usize,
    /// A reference to the [`Type`] of the field.
    pub vtable: IgnoredFieldVTable,
}

/// Whether a [`Struct`] describes a struct type or a tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructKind {
    /// A struct type, which has a name. This includes tuple structs (ex: `struct Foo(u32);`) and unit
    /// structs (ex: `struct Foo;`).
    Struct,
    /// A tuple (ex: `(u32, f32)`), which has no name. Tuples always have [`FieldKind::Unnamed`]
    /// fields. The unit tuple `()` is a [`Primitive`](crate::Primitive) instead.
    Tuple,
}

/// The "kind of fields" of a type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldKind {
    /// Fields have names (ex: `value.foo`, `value.bar`).
    /// Declared with `{}`.
    Named,
    /// Fields do not have names and are identified by their indices (ex: `value.0`, `value.1`).
    /// Declared with `()`.
    Unnamed,
    /// A field-less type.
    /// Declared with "nothing" (no `{}` and no `()`).
    Unit,
}

/// How the bytes of a struct that are not covered by a field are initialized when the struct is
/// constructed field-by-field (see [`Type::initialize_struct`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UncoveredBytes {
    /// The bytes are padding, which doesn't need to be initialized. This is the case for every
    /// struct whose fields are all described (including [ignored fields](Struct::ignored_fields)).
    Padding,
    /// The bytes are part of the value, and are valid when they are zeroed (ex: the 4th SIMD lane
    /// of `glam::Vec3A`, which has 3 fields). They are zeroed before the fields are initialized.
    Zeroed,
}

impl Struct {
    /// Create a new struct definition.
    ///
    /// # Safety
    ///
    /// Together, `fields` and `ignored_fields` must contain every field of the struct being described
    /// exactly once, each with the correct offset and type (see [`IgnoredField`] for the requirements
    /// of ignored fields).
    ///
    /// Bytes of the struct that are not covered by a field must be padding if `uncovered_bytes` is
    /// [`UncoveredBytes::Padding`], or valid when they are zeroed if it is
    /// [`UncoveredBytes::Zeroed`].
    ///
    /// # Panics
    ///
    /// Panics if `struct_kind` is [`StructKind::Tuple`] and `field_kind` is not
    /// [`FieldKind::Unnamed`].
    pub const unsafe fn new(
        fields: &'static [Field],
        field_kind: FieldKind,
        struct_kind: StructKind,
        ignored_fields: &'static [IgnoredField],
        uncovered_bytes: UncoveredBytes,
    ) -> Self {
        assert!(
            !matches!(struct_kind, StructKind::Tuple) || matches!(field_kind, FieldKind::Unnamed),
            "tuples must have unnamed fields"
        );
        Self {
            fields,
            field_kind,
            struct_kind,
            ignored_fields,
            uncovered_bytes,
        }
    }

    /// How the bytes of the struct that are not covered by a field are initialized.
    #[inline]
    pub const fn uncovered_bytes(&self) -> UncoveredBytes {
        self.uncovered_bytes
    }

    /// The fields of the struct.
    #[inline]
    pub const fn fields(&self) -> &'static [Field] {
        self.fields
    }

    /// Returns the index of the reflected field named `name` (in [`Struct::fields`]), or `None` if
    /// there is no such field. Unnamed fields are named by their index (ex: `"0"`), and
    /// [ignored fields](Struct::ignored_fields) are not included.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|field| field.name == name)
    }

    /// The [`FieldKind`] of the struct.
    #[inline]
    pub const fn field_kind(&self) -> &FieldKind {
        &self.field_kind
    }

    /// The [`StructKind`] of the struct.
    #[inline]
    pub const fn struct_kind(&self) -> StructKind {
        self.struct_kind
    }

    /// The fields that are not reflected (ex: fields marked `#[reflect(ignore)]`).
    /// These are missing from [`Struct::fields`].
    #[inline]
    pub const fn ignored_fields(&self) -> &'static [IgnoredField] {
        self.ignored_fields
    }

    /// Returns `true` if the struct has [ignored fields](Struct::ignored_fields).
    #[inline]
    pub const fn has_ignored_fields(&self) -> bool {
        !self.ignored_fields.is_empty()
    }
}

impl Field {
    /// Access the field for the given struct ptr.
    ///
    /// # Safety
    ///
    /// `self` must be a field of a [`Struct`] that describes the type of `struct_ptr`.
    pub unsafe fn access<'a>(&self, struct_ptr: Ptr<'a>) -> &'a dyn Reflect {
        // SAFETY: `struct_ptr` matches the parent struct of this field, so offsetting it produces a
        // valid pointer to a value of `self.ty`
        unsafe { self.ty.get().as_reflect(struct_ptr.byte_add(self.offset)) }
    }

    /// Mutably access the field for the given struct ptr.
    ///
    /// # Safety
    ///
    /// `self` must be a field of a [`Struct`] that describes the type of `struct_ptr`.
    pub unsafe fn access_mut<'a>(&self, struct_ptr: PtrMut<'a>) -> &'a mut dyn Reflect {
        // SAFETY: `struct_ptr` matches the parent struct of this field, so offsetting it produces a
        // valid, unique pointer to a value of `self.ty`
        unsafe {
            self.ty
                .get()
                .as_reflect_mut(struct_ptr.byte_add(self.offset))
        }
    }
}

/// A lazy reference to a [`Type`] via a function pointer. This makes evaluation lazy, which moves
/// const-eval to runtime, boosting compile times.
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

/// A reflected reference to a [`Struct`] value, providing read-only reflection operations on that value.
pub struct StructRef<'a> {
    // SAFETY NOTE: this should only ever be constructed with StructRef::new
    ptr: Ptr<'a>,
    // SAFETY NOTE: this should only ever be constructed with StructRef::new
    struct_type: &'static Struct,
}

impl<'a> StructRef<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `struct_type`
    pub unsafe fn new(ptr: Ptr<'a>, struct_type: &'static Struct) -> Self {
        Self { ptr, struct_type }
    }

    /// The [`Struct`] of this type.
    #[inline]
    pub fn struct_type(&self) -> &'static Struct {
        self.struct_type
    }

    /// Accesses the field at the given `index`. Returns [`None`] if the index is out of bounds.
    pub fn field(&self, index: usize) -> Option<&'a dyn Reflect> {
        let field = self.struct_type.fields.get(index)?;
        // SAFETY: self.ptr matches the type of the parent struct of this field
        Some(unsafe { field.access(self.ptr) })
    }

    /// Accesses the reflected field with `name` (see [`Struct::field_index`]).
    pub fn field_by_name(&self, name: &str) -> Option<&'a dyn Reflect> {
        self.field(self.struct_type.field_index(name)?)
    }

    /// Iterates over each field description and its value, in declaration order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (&'static Field, &'a dyn Reflect)> + Clone + use<'a> {
        let ptr = self.ptr;
        self.struct_type.fields.iter().map(move |field| {
            // SAFETY: ptr matches the type of the parent struct of this field
            (field, unsafe { field.access(ptr) })
        })
    }
}

/// A reflected mutable reference to a [`Struct`] value, providing reflection operations on that value.
pub struct StructMut<'a> {
    // SAFETY NOTE: this should only ever be constructed with StructMut::new
    ptr: PtrMut<'a>,
    // SAFETY NOTE: this should only ever be constructed with StructMut::new
    struct_type: &'static Struct,
}

impl<'a> StructMut<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `struct_type`
    pub unsafe fn new(ptr: PtrMut<'a>, struct_type: &'static Struct) -> Self {
        Self { ptr, struct_type }
    }

    /// The [`Struct`] of this type.
    #[inline]
    pub fn struct_type(&self) -> &'static Struct {
        self.struct_type
    }

    /// Returns a read-only view of this struct.
    #[inline]
    pub fn as_ref(&self) -> StructRef<'_> {
        StructRef {
            ptr: self.ptr.as_ref(),
            struct_type: self.struct_type,
        }
    }

    /// Accesses the field at the given `index`. Returns [`None`] if the index is out of bounds.
    #[inline]
    pub fn field(&self, index: usize) -> Option<&dyn Reflect> {
        self.as_ref().field(index)
    }

    /// Mutably accesses the field at the given `index`. Returns [`None`] if the index is out of bounds.
    pub fn field_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        let field = self.struct_type.fields.get(index)?;
        // SAFETY: self.ptr matches the type of the parent struct of this field. The returned borrow
        // is tied to `&mut self`, which prevents aliasing.
        Some(unsafe { field.access_mut(self.ptr.reborrow()) })
    }

    /// Accesses the reflected field with `name` (see [`Struct::field_index`]).
    #[inline]
    pub fn field_by_name(&self, name: &str) -> Option<&dyn Reflect> {
        self.as_ref().field_by_name(name)
    }

    /// Mutably accesses the reflected field with `name` (see [`Struct::field_index`]).
    pub fn field_by_name_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        self.field_mut(self.struct_type.field_index(name)?)
    }

    /// Iterates over each field description and its value, in declaration order.
    #[inline]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&'static Field, &dyn Reflect)> + Clone {
        self.as_ref().iter()
    }
}

/// Describes how to initialize a field that is not reflected. This allows values with ignored
/// fields to be constructed field-by-field (ex: [`Type::initialize_struct`] and structural clones).
///
/// The functions must be valid for pointers to the field's type, and `layout` must be the layout of
/// the field's type.
#[derive(Debug)]
pub struct IgnoredFieldVTable {
    /// The [`Layout`] of the field's type.
    pub layout: Layout,
    /// Writes a default value to the given (possibly uninitialized) pointer, without dropping the
    /// previous value (see [`set_default`](crate::set_default)). Registered with
    /// `#[reflect(default)]`.
    pub default: Option<unsafe fn(*mut u8)>,
    /// Writes a clone of the value at the first pointer to the second (possibly uninitialized)
    /// pointer, without dropping the previous value (see [`clone_into`](crate::clone_into)).
    /// Registered with `#[reflect(clone)]`.
    pub clone: Option<unsafe fn(Ptr<'_>, *mut u8)>,
}
