use crate::{Reflect, TypeRef};
use bevy_ptr::{Ptr, PtrMut};

/// Describes a fixed-size sequence of items that are laid out contiguously in memory (ex: `[T; N]`).
///
/// Unsafe code relies on the length and stride being accurate, so this can only be constructed
/// using [`Array::new`].
#[derive(Debug)]
pub struct Array {
    item: TypeRef,
    len: usize,
    stride: usize,
}

impl Array {
    /// Create a new array definition.
    ///
    /// # Safety
    ///
    /// The described type must contain `len` contiguous values of type `item`, where the item at
    /// index `i` starts `i * stride` bytes after the start of the array. Bytes that are not covered
    /// by an item must not contain other values.
    pub const unsafe fn new(item: TypeRef, len: usize, stride: usize) -> Self {
        Self { item, len, stride }
    }

    /// The type of each item in the array.
    #[inline]
    pub const fn item(&self) -> TypeRef {
        self.item
    }

    /// The number of items in the array.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if there are no items in the array. Otherwise returns `false`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The distance in bytes between the start of consecutive items.
    #[inline]
    pub const fn stride(&self) -> usize {
        self.stride
    }

    /// Takes an `array_ptr` to an array of this type and the `index` of an item in the array, and returns
    /// a pointer to that item. Will return [`None`] if the index is out of bounds.
    ///
    /// # Safety
    ///
    /// `array_ptr` must be a valid, initialized value that matches this [`Array`].
    unsafe fn item_ptr<'a>(&self, array_ptr: Ptr<'a>, index: usize) -> Option<Ptr<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: `index` is in bounds, so `Array::new` guarantees the offset is in bounds of the array
        Some(unsafe { array_ptr.byte_add(index * self.stride) })
    }
}

/// A reflected reference to an [`Array`] value, providing read-only reflection operations on that value.
#[derive(Clone, Copy)]
pub struct ArrayRef<'a> {
    // SAFETY NOTE: this should only ever be constructed with ArrayRef::new
    ptr: Ptr<'a>,
    // SAFETY NOTE: this should only ever be constructed with ArrayRef::new
    array_type: &'static Array,
}

impl<'a> ArrayRef<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `array_type`
    pub unsafe fn new(ptr: Ptr<'a>, array_type: &'static Array) -> Self {
        Self { ptr, array_type }
    }

    /// The [`Array`] of this type.
    #[inline]
    pub fn array_type(&self) -> &'static Array {
        self.array_type
    }

    /// The number of items in the array.
    #[inline]
    pub fn len(&self) -> usize {
        self.array_type.len
    }

    /// Returns `true` if there are no items in the array. Otherwise returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.array_type.is_empty()
    }

    /// Returns a reference to the reflected item in the array at `index`, if it exists. Otherwise returns [`None`].
    pub fn get(&self, index: usize) -> Option<&'a dyn Reflect> {
        // SAFETY: self.ptr matches `array_type`
        let item_ptr = unsafe { self.array_type.item_ptr(self.ptr, index)? };
        // SAFETY: `Array::new` guarantees that `item_ptr` is a valid value of the item type
        Some(unsafe { self.array_type.item.get().as_reflect(item_ptr) })
    }

    /// Iterates over all items in the array.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'a dyn Reflect> + Clone + use<'a> {
        let array = ArrayRef {
            ptr: self.ptr,
            array_type: self.array_type,
        };
        (0..self.len()).map(move |index| array.get(index).expect("index is less than len"))
    }
}

/// A reflected mutable reference to an [`Array`] value, providing read-only reflection operations on that value.
pub struct ArrayMut<'a> {
    // SAFETY NOTE: this should only ever be constructed with ArrayMut::new
    ptr: PtrMut<'a>,
    // SAFETY NOTE: this should only ever be constructed with ArrayMut::new
    array_type: &'static Array,
}

impl<'a> ArrayMut<'a> {
    /// Create a new instance.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, initialized value that exactly matches the type described in `array_type`
    pub unsafe fn new(ptr: PtrMut<'a>, array_type: &'static Array) -> Self {
        Self { ptr, array_type }
    }

    /// Returns a read-only view of this array.
    #[inline]
    pub fn as_ref(&self) -> ArrayRef<'_> {
        ArrayRef {
            ptr: self.ptr.as_ref(),
            array_type: self.array_type,
        }
    }

    /// The [`Array`] of this type.
    #[inline]
    pub fn array_type(&self) -> &'static Array {
        self.array_type
    }

    /// The number of items in the array.
    #[inline]
    pub fn len(&self) -> usize {
        self.array_type.len
    }

    /// Returns `true` if there are no items in the array. Otherwise returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.array_type.is_empty()
    }

    /// Returns a reference to the reflected item in the array at `index`, if it exists. Otherwise returns [`None`].
    #[inline]
    pub fn get(&self, index: usize) -> Option<&dyn Reflect> {
        self.as_ref().get(index)
    }

    /// Returns a mutable reference to the reflected item in the array at `index`, if it exists. Otherwise returns [`None`].
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        if index >= self.array_type.len {
            return None;
        }
        // SAFETY: `index` is in bounds, so `Array::new` guarantees that the offset points to a valid
        // value of the item type. The returned borrow is tied to `&mut self`, which prevents aliasing.
        unsafe {
            let item_ptr = self.ptr.reborrow().byte_add(index * self.array_type.stride);
            Some(self.array_type.item.get().as_reflect_mut(item_ptr))
        }
    }

    /// Iterates over all items in the array.
    #[inline]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &dyn Reflect> {
        self.as_ref().iter()
    }
}
