use std::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

/// Type-erased pointer into memory. Guaranteed to be correctly aligned, non-null and safe to read for a particular type.
#[derive(Copy, Clone)]
pub struct Ptr<'a>(NonNull<u8>, PhantomData<&'a u8>);

/// Type-erased pointer into memory. Guaranteed to be correctly aligned, non-null and safe to modify for a particular type.
pub struct PtrMut<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

/// Type-erased pointer into memory. Guaranteed to be correctly aligned, non-null and safe to move out of for a particular type.
pub struct OwningPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

pub struct ThinSlicePtr<'a, T> {
    ptr: NonNull<T>,
    #[cfg(debug_assertions)]
    len: usize,
    _marker: PhantomData<&'a [T]>,
}

impl<T> Clone for ThinSlicePtr<'_, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            #[cfg(debug_assertions)]
            len: self.len,
            _marker: PhantomData,
        }
    }
}
impl<T> Copy for ThinSlicePtr<'_, T> {}

impl<'a, T> ThinSlicePtr<'a, T> {
    pub fn new(slice: &'a [T]) -> Self {
        unsafe {
            Self {
                ptr: NonNull::new_unchecked(slice.as_ptr() as *mut _),
                #[cfg(debug_assertions)]
                len: slice.len(),
                _marker: PhantomData,
            }
        }
    }

    /// # Safety
    /// ptr must be valid for the returned lifetime.
    pub unsafe fn new_raw(ptr: NonNull<T>, #[cfg(debug_assertions)] len: usize) -> Self {
        Self {
            ptr,
            #[cfg(debug_assertions)]
            len,
            _marker: PhantomData,
        }
    }

    /// # Safety
    /// index must not be out of bounds
    pub unsafe fn index(self, index: usize) -> &'a T {
        debug_assert!(index < self.len);
        &*self.ptr.as_ptr().add(index)
    }

    /// # Safety
    /// index must not be out of bounds, and the same element must not be mutably accessed twice.
    pub unsafe fn index_mut(self, index: usize) -> &'a mut T {
        debug_assert!(index < self.len);
        &mut *self.ptr.as_ptr().add(index)
    }
}

macro_rules! impl_ptr {
    ($ptr:ident) => {
        impl $ptr<'_> {
            /// # Safety
            /// the offset cannot make the existing ptr null, or take it out of bounds for it's allocation.
            pub unsafe fn offset(self, count: isize) -> Self {
                Self(
                    NonNull::new_unchecked(self.0.as_ptr().offset(count)),
                    PhantomData,
                )
            }

            /// # Safety
            /// the offset cannot make the existing ptr null, or take it out of bounds for it's allocation.
            pub unsafe fn add(self, count: usize) -> Self {
                Self(
                    NonNull::new_unchecked(self.0.as_ptr().add(count)),
                    PhantomData,
                )
            }

            /// # Safety
            /// the lifetime for the returned item must not exceed the lifetime `inner` is valid for
            pub unsafe fn new(inner: NonNull<u8>) -> Self {
                Self(inner, PhantomData)
            }
        }
    };
}

impl_ptr!(Ptr);
impl<'a> Ptr<'a> {
    /// # Safety
    /// another PtrMut for the same Ptr shouldn't be created until the first is dropped.
    pub unsafe fn assert_unique(self) -> PtrMut<'a> {
        PtrMut(self.0, PhantomData)
    }

    /// # Safety
    /// Must point to a valid T
    pub unsafe fn deref<T>(self) -> &'a T {
        &*self.0.as_ptr().cast()
    }
}
impl_ptr!(PtrMut);
impl<'a> PtrMut<'a> {
    pub fn inner(self) -> NonNull<u8> {
        self.0
    }

    /// # Safety
    /// must have right to drop or move out of PtrMut, and current PtrMut should not be accessed again unless it's written to again.
    pub unsafe fn promote(self) -> OwningPtr<'a> {
        OwningPtr(self.0, PhantomData)
    }

    /// # Safety
    /// Must point to a valid T
    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        &mut *self.inner().as_ptr().cast()
    }
}
impl_ptr!(OwningPtr);
impl<'a> OwningPtr<'a> {
    pub fn inner(self) -> *mut u8 {
        self.0.as_ptr()
    }

    pub fn make<T, F: FnOnce(OwningPtr<'_>) -> R, R>(val: T, f: F) -> R {
        let mut temp = MaybeUninit::new(val);
        let ptr = unsafe { NonNull::new_unchecked(temp.as_mut_ptr().cast::<u8>()) };
        f(Self(ptr, PhantomData))
    }

    /// # Safety
    /// must point to a valid T.
    pub unsafe fn read<T>(self) -> T {
        self.inner().cast::<T>().read()
    }
}
