#![doc = include_str!("../README.md")]
#![no_std]
#![warn(missing_docs)]

use core::{
    cell::UnsafeCell, marker::PhantomData, mem::ManuallyDrop, num::NonZeroUsize, ptr::NonNull,
};

/// Type-erased borrow of some unknown type chosen when constructing this type.
///
/// This type tries to act "borrow-like" which means that:
/// - It should be considered immutable: its target must not be changed while this pointer is alive.
/// - It must always points to a valid value of whatever the pointee type is.
/// - The lifetime `'a` accurately represents how long the pointer is valid for.
///
/// It may be helpful to think of this type as similar to `&'a dyn Any` but without
/// the metadata and able to point to data that does not correspond to a Rust type.
#[derive(Copy, Clone)]
pub struct Ptr<'a>(NonNull<u8>, PhantomData<&'a u8>);

/// Type-erased mutable borrow of some unknown type chosen when constructing this type.
///
/// This type tries to act "borrow-like" which means that:
/// - Pointer is considered exclusive and mutable. It cannot be cloned as this would lead to
///   aliased mutability.
/// - It must always points to a valid value of whatever the pointee type is.
/// - The lifetime `'a` accurately represents how long the pointer is valid for.
///
/// It may be helpful to think of this type as similar to `&'a mut dyn Any` but without
/// the metadata and able to point to data that does not correspond to a Rust type.
pub struct PtrMut<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

/// Type-erased Box-like pointer to some unknown type chosen when constructing this type.
/// Conceptually represents ownership of whatever data is being pointed to and so is
/// responsible for calling its `Drop` impl. This pointer is _not_ responsible for freeing
/// the memory pointed to by this pointer as it may be pointing to an element in a `Vec` or
/// to a local in a function etc.
///
/// This type tries to act "borrow-like" like which means that:
/// - Pointer should be considered exclusive and mutable. It cannot be cloned as this would lead
///   to aliased mutability and potentially use after free bugs.
/// - It must always points to a valid value of whatever the pointee type is.
/// - The lifetime `'a` accurately represents how long the pointer is valid for.
///
/// It may be helpful to think of this type as similar to `&'a mut ManuallyDrop<dyn Any>` but
/// without the metadata and able to point to data that does not correspond to a Rust type.
pub struct OwningPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

macro_rules! impl_ptr {
    ($ptr:ident) => {
        impl $ptr<'_> {
            /// Calculates the offset from a pointer.
            /// As the pointer is type-erased, there is no size information available. The provided
            /// `count` parameter is in raw bytes.
            ///
            /// *See also: [`ptr::offset`][ptr_offset]*
            ///
            /// # Safety
            /// the offset cannot make the existing ptr null, or take it out of bounds for its allocation.
            ///
            /// [ptr_offset]: https://doc.rust-lang.org/std/primitive.pointer.html#method.offset
            #[inline]
            pub unsafe fn byte_offset(self, count: isize) -> Self {
                Self(
                    NonNull::new_unchecked(self.as_ptr().offset(count)),
                    PhantomData,
                )
            }

            /// Calculates the offset from a pointer (convenience for `.offset(count as isize)`).
            /// As the pointer is type-erased, there is no size information available. The provided
            /// `count` parameter is in raw bytes.
            ///
            /// *See also: [`ptr::add`][ptr_add]*
            ///
            /// # Safety
            /// the offset cannot make the existing ptr null, or take it out of bounds for its allocation.
            ///
            /// [ptr_add]: https://doc.rust-lang.org/std/primitive.pointer.html#method.add
            #[inline]
            pub unsafe fn byte_add(self, count: usize) -> Self {
                Self(
                    NonNull::new_unchecked(self.as_ptr().add(count)),
                    PhantomData,
                )
            }

            /// Creates a new instance from a raw pointer.
            ///
            /// # Safety
            /// The lifetime for the returned item must not exceed the lifetime `inner` is valid for
            #[inline]
            pub unsafe fn new(inner: NonNull<u8>) -> Self {
                Self(inner, PhantomData)
            }
        }
    };
}

impl_ptr!(Ptr);
impl_ptr!(PtrMut);
impl_ptr!(OwningPtr);

impl<'a> Ptr<'a> {
    /// Transforms this [`Ptr`] into an [`PtrMut`]
    ///
    /// # Safety
    /// Another [`PtrMut`] for the same [`Ptr`] must not be created until the first is dropped.
    #[inline]
    pub unsafe fn assert_unique(self) -> PtrMut<'a> {
        PtrMut(self.0, PhantomData)
    }

    /// Transforms this [`Ptr<T>`] into a `&T` with the same lifetime
    ///
    /// # Safety
    /// Must point to a valid `T`
    #[inline]
    pub unsafe fn deref<T>(self) -> &'a T {
        &*self.as_ptr().cast()
    }

    /// Gets the underlying pointer, erasing the associated lifetime.
    ///
    /// If possible, it is strongly encouraged to use [`deref`](Self::deref) over this function,
    /// as it retains the lifetime.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_ptr(self) -> *mut u8 {
        self.0.as_ptr()
    }
}

impl<'a, T> From<&'a T> for Ptr<'a> {
    #[inline]
    fn from(val: &'a T) -> Self {
        // SAFETY: The returned pointer has the same lifetime as the passed reference.
        // Access is immutable.
        unsafe { Self::new(NonNull::from(val).cast()) }
    }
}

impl<'a> PtrMut<'a> {
    /// Transforms this [`PtrMut`] into an [`OwningPtr`]
    ///
    /// # Safety
    /// Must have right to drop or move out of [`PtrMut`].
    #[inline]
    pub unsafe fn promote(self) -> OwningPtr<'a> {
        OwningPtr(self.0, PhantomData)
    }

    /// Transforms this [`PtrMut<T>`] into a `&mut T` with the same lifetime
    ///
    /// # Safety
    /// Must point to a valid `T`
    #[inline]
    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        &mut *self.as_ptr().cast()
    }

    /// Gets the underlying pointer, erasing the associated lifetime.
    ///
    /// If possible, it is strongly encouraged to use [`deref_mut`](Self::deref_mut) over
    /// this function, as it retains the lifetime.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }
}

impl<'a, T> From<&'a mut T> for PtrMut<'a> {
    #[inline]
    fn from(val: &'a mut T) -> Self {
        // SAFETY: The returned pointer has the same lifetime as the passed reference.
        // The reference is mutable, and thus will not alias.
        unsafe { Self::new(NonNull::from(val).cast()) }
    }
}

impl<'a> OwningPtr<'a> {
    /// Consumes a value and creates an [`OwningPtr`] to it while ensuring a double drop does not happen.
    #[inline]
    pub fn make<T, F: FnOnce(OwningPtr<'_>) -> R, R>(val: T, f: F) -> R {
        let mut temp = ManuallyDrop::new(val);
        // SAFETY: The value behind the pointer will not get dropped or observed later,
        // so it's safe to promote it to an owning pointer.
        f(unsafe { PtrMut::from(&mut *temp).promote() })
    }

    /// Consumes the [`OwningPtr`] to obtain ownership of the underlying data of type `T`.
    ///
    /// # Safety
    /// Must point to a valid `T`.
    #[inline]
    pub unsafe fn read<T>(self) -> T {
        self.as_ptr().cast::<T>().read()
    }

    /// Consumes the [`OwningPtr`] to drop the underlying data of type `T`.
    ///
    /// # Safety
    /// Must point to a valid `T`.
    #[inline]
    pub unsafe fn drop_as<T>(self) {
        self.as_ptr().cast::<T>().drop_in_place();
    }

    /// Gets the underlying pointer, erasing the associated lifetime.
    ///
    /// If possible, it is strongly encouraged to use the other more type-safe functions
    /// over this function.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }
}

/// Conceptually equivalent to `&'a [T]` but with length information cut out for performance reasons
pub struct ThinSlicePtr<'a, T> {
    ptr: NonNull<T>,
    #[cfg(debug_assertions)]
    len: usize,
    _marker: PhantomData<&'a [T]>,
}

impl<'a, T> ThinSlicePtr<'a, T> {
    #[inline]
    /// Indexes the slice without doing bounds checks
    ///
    /// # Safety
    /// `index` must be in-bounds.
    pub unsafe fn get(self, index: usize) -> &'a T {
        #[cfg(debug_assertions)]
        debug_assert!(index < self.len);

        &*self.ptr.as_ptr().add(index)
    }
}

impl<'a, T> Clone for ThinSlicePtr<'a, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            #[cfg(debug_assertions)]
            len: self.len,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Copy for ThinSlicePtr<'a, T> {}

impl<'a, T> From<&'a [T]> for ThinSlicePtr<'a, T> {
    #[inline]
    fn from(slice: &'a [T]) -> Self {
        Self {
            // SAFETY: a reference can never be null
            ptr: unsafe { NonNull::new_unchecked(slice.as_ptr() as *mut T) },
            #[cfg(debug_assertions)]
            len: slice.len(),
            _marker: PhantomData,
        }
    }
}

/// Creates a dangling pointer with specified alignment.
/// See [`NonNull::dangling`].
pub fn dangling_with_align(align: NonZeroUsize) -> NonNull<u8> {
    // SAFETY: The pointer will not be null, since it was created
    // from the address of a `NonZeroUsize`.
    unsafe { NonNull::new_unchecked(align.get() as *mut u8) }
}

mod private {
    use core::cell::UnsafeCell;

    pub trait SealedUnsafeCell {}
    impl<'a, T> SealedUnsafeCell for &'a UnsafeCell<T> {}
}

/// Extension trait for helper methods on [`UnsafeCell`]
pub trait UnsafeCellDeref<'a, T>: private::SealedUnsafeCell {
    /// # Safety
    /// - The returned value must be unique and not alias any mutable or immutable references to the contents of the [`UnsafeCell`].
    /// - At all times, you must avoid data races. If multiple threads have access to the same [`UnsafeCell`], then any writes must have a proper happens-before relation to all other accesses or use atomics ([`UnsafeCell`] docs for reference).
    unsafe fn deref_mut(self) -> &'a mut T;

    /// # Safety
    /// - For the lifetime `'a` of the returned value you must not construct a mutable reference to the contents of the [`UnsafeCell`].
    /// - At all times, you must avoid data races. If multiple threads have access to the same [`UnsafeCell`], then any writes must have a proper happens-before relation to all other accesses or use atomics ([`UnsafeCell`] docs for reference).
    unsafe fn deref(self) -> &'a T;

    /// Returns a copy of the contained value.
    ///
    /// # Safety
    /// - The [`UnsafeCell`] must not currently have a mutable reference to its content.
    /// - At all times, you must avoid data races. If multiple threads have access to the same [`UnsafeCell`], then any writes must have a proper happens-before relation to all other accesses or use atomics ([`UnsafeCell`] docs for reference).
    unsafe fn read(self) -> T
    where
        T: Copy;
}

impl<'a, T> UnsafeCellDeref<'a, T> for &'a UnsafeCell<T> {
    #[inline]
    unsafe fn deref_mut(self) -> &'a mut T {
        &mut *self.get()
    }
    #[inline]
    unsafe fn deref(self) -> &'a T {
        &*self.get()
    }

    #[inline]
    unsafe fn read(self) -> T
    where
        T: Copy,
    {
        self.get().read()
    }
}
