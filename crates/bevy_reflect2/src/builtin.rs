//! Reflected versions of core traits. See [`Reflect`] for details.

use crate::{FieldKind, IgnoredFieldVTable, KindRef, Reflect, StructKind, Type, UncoveredBytes};
use alloc::{alloc::dealloc, boxed::Box, vec::Vec};
use bevy_platform::hash::FixedHasher;
use core::{
    alloc::Layout,
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    hash::{BuildHasher, Hash, Hasher},
    mem,
    ptr::{self, NonNull},
};

impl dyn Reflect {
    /// Returns a clone of this value using its [`Clone`] implementation, falling back to a
    /// structural implementation if it doesn't have one. Returns `None` if any part of it cannot be
    /// cloned.
    pub fn reflect_clone(&self) -> Option<Box<dyn Reflect>> {
        let ty = self.ty();
        if let Some(clone) = ty.vtable().clone {
            // SAFETY: `Reflect::ty` accurately describes this value
            return unsafe { clone(self.ptr()) };
        }
        match self.kind_ref() {
            KindRef::Struct(value) => {
                let struct_type = value.struct_type();
                let size = size_of_val(self);
                let ignored_fields = struct_type
                    .ignored_fields()
                    .iter()
                    .map(|field| (field.offset, &field.vtable));
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
                            (offset, &field.vtable)
                        });
                // The discriminant is stored outside of the fields (unless it is niche-encoded), so
                // the bytes that are not covered by a field are copied from `self`
                let mut ranges = fields
                    .clone()
                    .map(|(offset, field)| (offset, size_of_val(field)))
                    .chain(
                        ignored_fields
                            .clone()
                            .map(|(offset, vtable)| (offset, vtable.layout.size())),
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
            KindRef::List(value) => {
                let mut clone = ty.default()?;
                let mut list = clone.list_mut()?;
                for item in value.iter() {
                    list.push(item.reflect_clone()?).ok()?;
                }
                Some(clone)
            }
            KindRef::Map(value) => {
                let mut clone = ty.default()?;
                let mut map = clone.map_mut()?;
                for (key, value) in value.iter() {
                    map.insert(key.reflect_clone()?, value.reflect_clone()?)
                        .ok()?;
                }
                Some(clone)
            }
            KindRef::Set(value) => {
                let mut clone = ty.default()?;
                let mut set = clone.set_mut()?;
                for item in value.iter() {
                    set.insert(item.reflect_clone()?).ok()?;
                }
                Some(clone)
            }
            // Primitives that don't have a reflected Clone impl cannot be cloned
            KindRef::Primitive(_) => None,
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
    /// Returns `None` if a field can't be cloned, including ignored fields without a `clone`
    /// function. Fields that were already cloned are dropped, and the allocation is freed.
    ///
    /// The clone has the same type (and variant ... in the case of enums) as `self`, so it has the
    /// same field offsets.
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
        ignored_fields: impl Iterator<Item = (usize, &'a IgnoredFieldVTable)> + Clone,
    ) -> Option<Box<dyn Reflect>> {
        // Check ignored fields before allocating. Their clone functions can't fail, so once these
        // checks pass, only reflected fields can fail.
        if ignored_fields
            .clone()
            .any(|(_, vtable)| vtable.clone.is_none())
        {
            return None;
        }

        let ty = self.ty();
        let dst = ty.alloc()?;
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
                return None;
            }
            // SAFETY: the caller guarantees that `offset` is the offset of `field` in `self`, which
            // is the same in `dst`. `clone` was checked to have the same type as `field`.
            unsafe { move_box_into(clone, dst.as_ptr().add(offset)) };
            guard.initialized += 1;
        }
        // If an ignored field's clone function panics, the ignored fields that were already cloned
        // are leaked, which is safe
        for (offset, vtable) in ignored_fields {
            let clone = vtable
                .clone
                .expect("ignored fields were checked to have `clone`");
            // SAFETY: the caller guarantees that `offset` is the offset of an ignored field in both
            // `self` and `dst`, and that `clone` is valid for its type
            unsafe { clone(self.ptr().byte_add(offset), dst.as_ptr().add(offset)) };
        }
        mem::forget(guard);
        // SAFETY: `dst` was allocated with `Type::alloc`. The caller guarantees that `fields` and
        // `ignored_fields` yielded every field of `self`, so every field was initialized above, and
        // that `init_uncovered` initialized every other byte, which makes `dst` a valid, fully owned
        // value.
        Some(unsafe { ty.box_from_raw(dst) })
    }

    /// Formats this value using its [`Debug`] implementation, falling back to a structural
    /// implementation if it doesn't have one. Values that cannot be formatted structurally print
    /// their type path.
    ///
    /// [`Debug`] is also implemented for `dyn Reflect` using this method.
    pub fn reflect_debug(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let ty = self.ty();
        if let Some(debug) = ty.vtable().debug {
            // SAFETY: `Reflect::ty` accurately describes this value
            return unsafe { debug(self.ptr(), f) };
        }
        match self.kind_ref() {
            KindRef::Struct(value) => {
                // Tuples are formatted like their `Debug` implementation (ex: `(1, 2.0)`). An empty
                // name formats like a tuple, including the trailing comma of 1-tuples.
                let name = match value.struct_type().struct_kind() {
                    StructKind::Struct => ty.type_path().short(),
                    StructKind::Tuple => "",
                };
                match value.struct_type().field_kind() {
                    FieldKind::Named => {
                        let mut debug = f.debug_struct(name);
                        for (field, value) in value.iter() {
                            debug.field(field.name, &value);
                        }
                        if value.struct_type().has_ignored_fields() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unnamed => {
                        let mut debug = f.debug_tuple(name);
                        for (_, value) in value.iter() {
                            debug.field(&value);
                        }
                        if value.struct_type().has_ignored_fields() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unit => f.write_str(name),
                }
            }
            KindRef::Enum(value) => {
                let variant = value.variant_type();
                match variant.field_kind {
                    FieldKind::Named => {
                        let mut debug = f.debug_struct(variant.name);
                        for (field, value) in value.iter() {
                            debug.field(field.name, &value);
                        }
                        if !variant.ignored_fields.is_empty() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unnamed => {
                        let mut debug = f.debug_tuple(variant.name);
                        for (_, value) in value.iter() {
                            debug.field(&value);
                        }
                        if !variant.ignored_fields.is_empty() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unit => f.write_str(variant.name),
                }
            }
            KindRef::List(value) => f.debug_list().entries(value.iter()).finish(),
            KindRef::Array(value) => f.debug_list().entries(value.iter()).finish(),
            KindRef::Map(value) => f.debug_map().entries(value.iter()).finish(),
            KindRef::Set(value) => f.debug_set().entries(value.iter()).finish(),
            KindRef::Primitive(_) => write!(f, "Reflect({})", ty.type_path().full()),
        }
    }

    /// Returns a hash of this value, or `None` if any part of it cannot be hashed.
    ///
    /// See [`Self::reflect_hash_into`].
    pub fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = FixedHasher.build_hasher();
        self.reflect_hash_into(&mut hasher)?;
        Some(hasher.finish())
    }

    /// Feeds this value into `hasher`. Returns `None` if any part of it cannot be hashed, in which
    /// case `hasher` may have been partially written to.
    ///
    /// This will use the reflected [`Hash`] implementation, if one is present. If not, it will rely
    /// on a "structural" hash using the reflected type information, which hashes the same values
    /// that the structural [`reflect_partial_eq`](Self::reflect_partial_eq) compares. Values with a
    /// reflected [`PartialEq`] implementation but no reflected [`Hash`] implementation cannot be
    /// hashed, because a structural hash could disagree with their equality.
    ///
    /// Values of different types produce different hashes, even if their contents are the same.
    pub fn reflect_hash_into(&self, mut hasher: &mut dyn Hasher) -> Option<()> {
        let ty = self.ty();
        ty.type_id().hash(&mut hasher);
        if let Some(hash) = ty.vtable().hash {
            // SAFETY: `Reflect::ty` accurately describes this value
            unsafe { hash(self.ptr(), hasher) };
            return Some(());
        }
        // Values that are equal according to a custom `PartialEq` (ex: case-insensitive strings) could
        // have different structural hashes
        if ty.vtable().partial_eq.is_some() {
            return None;
        }
        match self.kind_ref() {
            // Values of the same struct type have the same number of fields, so it isn't hashed.
            // Ignored fields are not hashed, because they are not compared.
            KindRef::Struct(value) => {
                for (_, field) in value.iter() {
                    field.reflect_hash_into(hasher)?;
                }
            }
            KindRef::Enum(value) => {
                hasher.write_usize(value.variant());
                for (_, field) in value.iter() {
                    field.reflect_hash_into(hasher)?;
                }
            }
            KindRef::List(value) => {
                hasher.write_usize(value.len());
                for item in value.iter() {
                    item.reflect_hash_into(hasher)?;
                }
            }
            KindRef::Array(value) => {
                hasher.write_usize(value.len());
                for item in value.iter() {
                    item.reflect_hash_into(hasher)?;
                }
            }
            KindRef::Map(_) | KindRef::Set(_) | KindRef::Primitive(_) => {
                return None;
            }
        }
        Some(())
    }

    /// Returns whether this value is equal to `other`, or `None` if they cannot be compared.
    ///
    /// This will use the reflected [`PartialEq`] implementation, if one is present. If not, it will rely
    /// on a "structural" partial equality using the reflected type information.
    ///
    /// Values of different types are never equal.
    pub fn reflect_partial_eq(&self, other: &dyn Reflect) -> Option<bool> {
        let ty = self.ty();
        if ty.type_id() != other.ty().type_id() {
            return Some(false);
        }
        if let Some(partial_eq) = ty.vtable().partial_eq {
            // SAFETY: `Reflect::ty` accurately describes both values, which have the same type
            return Some(unsafe { partial_eq(self.ptr(), other.ptr()) });
        }
        match (self.kind_ref(), other.kind_ref()) {
            (KindRef::Struct(a), KindRef::Struct(b)) => {
                all_eq(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
            }
            (KindRef::Enum(a), KindRef::Enum(b)) => {
                if a.variant() != b.variant() {
                    return Some(false);
                }
                all_eq(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
            }
            (KindRef::List(a), KindRef::List(b)) => {
                if a.len() != b.len() {
                    return Some(false);
                }
                all_eq(a.iter(), b.iter())
            }
            (KindRef::Array(a), KindRef::Array(b)) => all_eq(a.iter(), b.iter()),
            (KindRef::Map(a), KindRef::Map(b)) => {
                if a.len() != b.len() {
                    return Some(false);
                }
                for (key, a_value) in a.iter() {
                    let Some(b_value) = b.get(key) else {
                        return Some(false);
                    };
                    if !a_value.reflect_partial_eq(b_value)? {
                        return Some(false);
                    }
                }
                Some(true)
            }
            (KindRef::Set(a), KindRef::Set(b)) => {
                Some(a.len() == b.len() && a.iter().all(|value| b.contains(value)))
            }
            _ => None,
        }
    }

    /// Returns an ordering between this value and `other`, or `None` if they cannot be compared.
    ///
    /// This will use the reflected [`PartialOrd`] implementation, if one is present. If not, it will rely
    /// on a "structural" partial ordering using the reflected type information.
    ///
    /// Values of different types cannot be compared.
    pub fn reflect_partial_cmp(&self, other: &dyn Reflect) -> Option<Ordering> {
        let ty = self.ty();
        if ty.type_id() != other.ty().type_id() {
            return None;
        }
        if let Some(partial_cmp) = ty.vtable().partial_cmp {
            // SAFETY: `Reflect::ty` accurately describes both values, which have the same type
            return unsafe { partial_cmp(self.ptr(), other.ptr()) };
        }
        match (self.kind_ref(), other.kind_ref()) {
            (KindRef::Struct(a), KindRef::Struct(b)) => {
                lexicographic_cmp(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
            }
            (KindRef::Enum(a), KindRef::Enum(b)) => match a.variant().cmp(&b.variant()) {
                Ordering::Equal => {
                    lexicographic_cmp(a.iter().map(|(_, v)| v), b.iter().map(|(_, v)| v))
                }
                ordering => Some(ordering),
            },
            (KindRef::List(a), KindRef::List(b)) => lexicographic_cmp(a.iter(), b.iter()),
            (KindRef::Array(a), KindRef::Array(b)) => lexicographic_cmp(a.iter(), b.iter()),
            _ => None,
        }
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

/// Cleans up a partially cloned value (see [`Reflect::clone_fields`]): drops the first
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

impl Debug for dyn Reflect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.reflect_debug(f)
    }
}

/// Returns whether every pair of values is equal. Both iterators must have the same length.
fn all_eq<'a>(
    a: impl Iterator<Item = &'a dyn Reflect>,
    b: impl Iterator<Item = &'a dyn Reflect>,
) -> Option<bool> {
    for (a, b) in a.zip(b) {
        if !a.reflect_partial_eq(b)? {
            return Some(false);
        }
    }
    Some(true)
}

/// Compares the values in order, using the first non-equal ordering. If one sequence is a prefix of
/// the other, the shorter sequence is ordered first.
fn lexicographic_cmp<'a>(
    mut a: impl Iterator<Item = &'a dyn Reflect>,
    mut b: impl Iterator<Item = &'a dyn Reflect>,
) -> Option<Ordering> {
    loop {
        match (a.next(), b.next()) {
            (Some(a), Some(b)) => match a.reflect_partial_cmp(b)? {
                Ordering::Equal => {}
                ordering => return Some(ordering),
            },
            (Some(_), None) => return Some(Ordering::Greater),
            (None, Some(_)) => return Some(Ordering::Less),
            (None, None) => return Some(Ordering::Equal),
        }
    }
}
