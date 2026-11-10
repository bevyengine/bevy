use crate::{kind::IgnoredFieldVTable, kind::KindRef, kind::UncoveredBytes, Reflect, Type};
use alloc::{alloc::dealloc, boxed::Box, vec::Vec};
use core::{
    alloc::Layout,
    mem,
    ptr::{self, NonNull},
};
use thiserror::Error;

impl dyn Reflect {
    /// Returns a clone of this value using its [`Clone`] implementation, falling back to a
    /// structural implementation if it doesn't have one. Returns an error describing the (possibly
    /// nested) value that failed if any part of it cannot be cloned.
    pub fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        use ReflectCloneErrorReason::*;
        let ty = self.ty();
        let error = |reason| ReflectCloneError::new(ty, reason);
        if let Some(clone) = ty.vtable().clone {
            // SAFETY: `Reflect::ty` accurately describes this value
            return unsafe { clone(self.ptr()) }.ok_or(error(CloneFailed));
        }
        match self.kind_ref() {
            KindRef::Struct(value) => {
                let struct_type = value.struct_type();
                let size = size_of_val(self);
                let ignored_fields = struct_type
                    .ignored_fields()
                    .iter()
                    .map(|field| (field.offset, field.name, &field.vtable));
                let init_uncovered = |dst: *mut u8| {
                    if struct_type.uncovered_bytes() == UncoveredBytes::Zeroed {
                        // SAFETY: `dst` is a new allocation of `size` bytes
                        unsafe { ptr::write_bytes(dst, 0, size) };
                    }
                };
                // SAFETY: `Struct::new` guarantees that the reflected and ignored fields are every
                // field of `self`, with accurate offsets, and that the bytes not covered by a field
                // are padding, or valid when zeroed if they are `UncoveredBytes::Zeroed`
                unsafe {
                    self.clone_fields(
                        init_uncovered,
                        value.iter().map(|(field, value)| (field.offset, value)),
                        ignored_fields,
                    )
                }
            }
            KindRef::Enum(value) => {
                // Enum variants don't describe their field offsets (`offset_of!` doesn't support
                // enums on stable), so they are found using `EnumAccess`. Values of the same type and
                // variant have the same field offsets.
                let src = self.ptr().as_ptr();
                let size = size_of_val(self);
                let fields = value.iter().enumerate().map(|(index, (_, field))| {
                    let offset = value
                        .field_offset(index)
                        .expect("index is a valid field of the variant");
                    (offset, field)
                });
                let ignored_fields =
                    value
                        .variant_type()
                        .ignored_fields
                        .iter()
                        .enumerate()
                        .map(|(index, field)| {
                            let offset = value
                                .ignored_field_offset(index)
                                .expect("index is a valid ignored field of the variant");
                            (offset, field.name, &field.vtable)
                        });
                // The discriminant is stored outside of the fields (unless it is niche-encoded), so
                // the bytes that are not covered by a field are copied from `self`
                let mut ranges = fields
                    .clone()
                    .map(|(offset, field)| (offset, size_of_val(field)))
                    .chain(
                        ignored_fields
                            .clone()
                            .map(|(offset, _, vtable)| (offset, vtable.layout.size())),
                    )
                    .collect::<Vec<_>>();
                // SAFETY: `dst` is a new allocation of `self`'s type, so it is valid for writes of
                // `size` bytes and doesn't overlap `self`. `EnumAccess` guarantees that the field
                // offsets are accurate, so every range is in bounds.
                let init_uncovered =
                    |dst: *mut u8| unsafe { copy_uncovered_bytes(src, dst, size, &mut ranges) };
                // SAFETY: `Enum::new` guarantees that the reflected and ignored fields are every field
                // of the active variant, and `EnumAccess` guarantees that their offsets are accurate,
                // so `init_uncovered` copies every byte that is not covered by a field.
                unsafe { self.clone_fields(init_uncovered, fields, ignored_fields) }
            }
            KindRef::Array(value) => {
                let stride = value.array_type().stride();
                // SAFETY: `Array::new` guarantees that the array contains exactly these items, with
                // the item at index `i` at offset `i * stride`. Arrays have no bytes other than their
                // items.
                unsafe {
                    self.clone_fields(
                        |_| {},
                        value.iter().enumerate().map(|(i, item)| (i * stride, item)),
                        core::iter::empty(),
                    )
                }
            }
            // A default value has the same type as `self`, so it has the same kind
            KindRef::List(value) => {
                let mut clone = ty.default().ok_or(error(MissingDefault))?;
                let list = clone.list_mut().expect("a default list is a list");
                for item in value.iter() {
                    list.push(item.reflect_clone()?)
                        .map_err(|_| error(InsertFailed))?;
                }
                Ok(clone)
            }
            KindRef::Map(value) => {
                let mut clone = ty.default().ok_or(error(MissingDefault))?;
                let map = clone.map_mut().expect("a default map is a map");
                for (key, value) in value.iter() {
                    map.insert(key.reflect_clone()?, value.reflect_clone()?)
                        .map_err(|_| error(InsertFailed))?;
                }
                Ok(clone)
            }
            KindRef::Set(value) => {
                let mut clone = ty.default().ok_or(error(MissingDefault))?;
                let set = clone.set_mut().expect("a default set is a set");
                for item in value.iter() {
                    set.insert(item.reflect_clone()?)
                        .map_err(|_| error(InsertFailed))?;
                }
                Ok(clone)
            }
            // Primitives that don't have a reflected Clone impl cannot be cloned
            KindRef::Primitive(_) => Err(error(NotCloneable)),
        }
    }

    /// Clones a value by cloning each of its `fields` directly into a new allocation.
    /// `ignored_fields` are cloned using their [`IgnoredFieldVTable::clone`] function. Before any
    /// field is written, `init_uncovered` is called with the allocation to initialize the bytes that
    /// are not covered by a field (ex: an enum discriminant).
    ///
    /// Field bytes are never read directly from self (only cloned), as reading those bytes could result
    /// in a data race in cases like atomics.
    ///
    /// Returns an error if a field can't be cloned, including ignored fields without a `clone`
    /// function. Fields that were already cloned are dropped, and the allocation is freed.
    ///
    /// The clone has the same type (and variant ... in the case of enums) as `self`, so it has the
    /// same field offsets.
    ///
    // If an ignored field's clone function panics, the ignored fields that were already cloned
    // are leaked, which is safe.
    ///
    /// # Safety
    ///
    /// Together, `fields` and `ignored_fields` must yield every field of `self` (ex: every field of
    /// the active variant of an enum) exactly once, each paired with its offset in bytes from the
    /// start of `self`. Each [`IgnoredFieldVTable`] must describe the type of its field.
    /// `init_uncovered` must leave every byte that is not covered by a field valid for `self`'s type
    /// (and variant), without writing to field bytes.
    unsafe fn clone_fields<'a>(
        &'a self,
        init_uncovered: impl FnOnce(*mut u8),
        fields: impl Iterator<Item = (usize, &'a dyn Reflect)> + Clone,
        ignored_fields: impl Iterator<Item = (usize, &'static str, &'a IgnoredFieldVTable)> + Clone,
    ) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        let ty = self.ty();
        // Check ignored fields before allocating. Their clone functions can't fail, so once these
        // checks pass, only reflected fields can fail.
        if let Some((_, name, _)) = ignored_fields
            .clone()
            .find(|(_, _, vtable)| vtable.clone.is_none())
        {
            return Err(ReflectCloneError::new(
                ty,
                ReflectCloneErrorReason::IgnoredFieldNoClone { name },
            ));
        }

        let dst = ty
            .alloc()
            .ok_or(ReflectCloneError::new(ty, ReflectCloneErrorReason::Unsized))?;
        // Drops the initialized fields and frees `dst` if a field fails to clone, or panics
        let mut guard = PartialClone {
            ty,
            dst,
            fields: fields.clone(),
            initialized: 0,
        };
        init_uncovered(dst.as_ptr());
        for (offset, field) in fields {
            let clone = field.reflect_clone()?;
            // A clone of a different type can't be moved into the field
            if clone.ty().type_id() != field.ty().type_id() {
                return Err(ReflectCloneError::new(
                    field.ty(),
                    ReflectCloneErrorReason::TypeMismatch { clone: clone.ty() },
                ));
            }
            // SAFETY: the caller guarantees that `offset` is the offset of `field` in `self`, which
            // is the same in `dst`. `clone` was checked to have the same type as `field`.
            unsafe { move_box_into(clone, dst.as_ptr().add(offset)) };
            guard.initialized += 1;
        }
        // If an ignored field's clone function panics, the ignored fields that were already cloned
        // are leaked, which is safe
        for (offset, _, vtable) in ignored_fields {
            // ignored fields were checked to have `clone`
            let clone = vtable.clone.unwrap();
            // SAFETY: the caller guarantees that `offset` is the offset of an ignored field in both
            // `self` and `dst`, and that `clone` is valid for its type
            unsafe { clone(self.ptr().byte_add(offset), dst.as_ptr().add(offset)) };
        }
        mem::forget(guard);
        // SAFETY: `dst` was allocated with `Type::alloc`. The caller guarantees that `fields` and
        // `ignored_fields` yielded every field of `self`, so every field was initialized above, and
        // that `init_uncovered` initialized every other byte (including the discriminant in the case
        // of enums), which makes `dst` a valid, fully owned value.
        Ok(unsafe { ty.box_from_raw(dst) })
    }
}

/// An error returned by [`<dyn Reflect>::reflect_clone`](trait.Reflect.html#method.reflect_clone)
/// when part of a value cannot be cloned.
#[derive(Error, Debug, Clone, Copy)]
#[error("cannot clone `{}`: {reason}", .ty.type_path().full())]
pub struct ReflectCloneError {
    /// The type of the (possibly nested) value that could not be cloned.
    pub ty: &'static Type,
    /// Why the value could not be cloned.
    pub reason: ReflectCloneErrorReason,
}

/// Why a value could not be cloned. See [`ReflectCloneError`].
#[derive(Error, Debug, Clone, Copy)]
pub enum ReflectCloneErrorReason {
    /// The type is a [`Primitive`](crate::kind::Primitive) without a reflected [`Clone`]
    /// implementation, so it cannot be cloned structurally.
    #[error("it has no reflected `Clone` implementation and cannot be cloned structurally")]
    NotCloneable,
    /// The type's reflected [`Clone`] function failed.
    #[error("its reflected `Clone` function failed")]
    CloneFailed,
    /// The type is unsized, so a clone cannot be allocated.
    #[error("it is unsized")]
    Unsized,
    /// The type has an ignored field without a clone function (registered with
    /// `#[reflect(ignore, clone)]`).
    #[error("ignored field `{name}` has no clone function")]
    IgnoredFieldNoClone {
        /// The name of the ignored field.
        name: &'static str,
    },
    /// The type is a list, map, or set without a reflected [`Default`] implementation. These are
    /// cloned structurally by inserting cloned items into a default value.
    #[error("it has no reflected `Default` implementation to clone its items into")]
    MissingDefault,
    /// The type's reflected [`Clone`] function returned a value of a different type.
    #[error("its clone has the different type `{}`", .clone.type_path().full())]
    TypeMismatch {
        /// The type of the returned clone.
        clone: &'static Type,
    },
    /// The type is a list, map, or set that rejected one of its cloned items.
    #[error("a cloned item could not be inserted into it")]
    InsertFailed,
}

impl ReflectCloneError {
    fn new(ty: &'static Type, reason: ReflectCloneErrorReason) -> Self {
        Self { ty, reason }
    }
}

/// Moves `value` into `dst` and deallocates its box without dropping it.
///
/// # Safety
///
/// `dst` must be valid for writes of a value of `value`'s type, and must not overlap `value`. Any
/// existing value at `dst` is overwritten without being dropped.
pub(crate) unsafe fn move_box_into(value: Box<dyn Reflect>, dst: *mut u8) {
    let layout = Layout::for_value(&*value);
    let src = Box::into_raw(value).cast::<u8>();
    // SAFETY: `src` is a valid value of `layout.size()` bytes, and the caller guarantees that `dst`
    // is valid for writes of the same type
    unsafe { ptr::copy_nonoverlapping(src, dst, layout.size()) };
    if layout.size() != 0 {
        // SAFETY: `src` was allocated by `Box` with the global allocator using `layout`. The value
        // was moved to `dst`, so it is deallocated without being dropped.
        unsafe { dealloc(src, layout) };
    }
}

/// Copies the bytes of `src` that are not covered by any of `fields` to `dst`.
///
/// Each field in `fields` is passed in as an `(offset, size)` pair, which may be in any order.
/// These will be sorted by their offset, which then enables finding the "gaps" between them.
///
/// Field bytes themselves are never read from `src` (only the uncovered bytes are read), because
/// a racing write to a field with interior mutability (ex: an atomic) would make the read a data race.
/// The data from fields themselves is copied elsewhere using a proper "clone".
///
/// # Safety
///
/// `src` must be valid for reads of `size` bytes, `dst` must be valid for writes of `size` bytes,
/// and they must not overlap. Every field must be in bounds of `size`.
unsafe fn copy_uncovered_bytes(
    src: *const u8,
    dst: *mut u8,
    size: usize,
    fields: &mut [(usize, usize)],
) {
    fields.sort_unstable_by_key(|(offset, _)| *offset);
    let mut start = 0;
    for &(offset, len) in &*fields {
        // Zero-sized fields can share an offset with (or lie inside) another field, so `offset`
        // may be before the end of the previous field
        if offset > start {
            // SAFETY: the caller guarantees that `start..offset` is in bounds of both `src` and
            // `dst`, which don't overlap
            unsafe { ptr::copy_nonoverlapping(src.add(start), dst.add(start), offset - start) };
        }
        start = start.max(offset + len);
    }
    if size > start {
        // SAFETY: the caller guarantees that `start..size` is in bounds of both `src` and `dst`,
        // which don't overlap
        unsafe { ptr::copy_nonoverlapping(src.add(start), dst.add(start), size - start) };
    }
}

/// Cleans up a partially cloned value (see `<dyn Reflect>::clone_fields`): drops the first
/// `initialized` `fields`, which were cloned into `dst`, and frees `dst`.
struct PartialClone<'a, I: Iterator<Item = (usize, &'a dyn Reflect)>> {
    ty: &'static Type,
    dst: NonNull<u8>,
    fields: I,
    initialized: usize,
}

impl<'a, I: Iterator<Item = (usize, &'a dyn Reflect)>> Drop for PartialClone<'a, I> {
    fn drop(&mut self) {
        for (offset, field) in self.fields.by_ref().take(self.initialized) {
            let field_ptr = field
                .ty()
                .as_reflect_ptr(self.dst.as_ptr().wrapping_add(offset));
            // SAFETY: the first `initialized` fields were moved into `dst` at their offset, with the
            // type of the field they were cloned from, and are owned by `dst`
            unsafe { ptr::drop_in_place(field_ptr.cast_mut()) };
        }
        // SAFETY: `dst` was allocated with `Type::alloc`, and its initialized fields were dropped
        unsafe { self.ty.dealloc(self.dst) };
    }
}
