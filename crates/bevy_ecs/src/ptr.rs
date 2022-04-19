use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

/// Type-erased pointer into memory. Guaranteed to be correctly aligned, non-null and safe to read for a particular type.
#[derive(Copy, Clone)]
pub struct Ptr<'a>(NonNull<u8>, PhantomData<&'a u8>);

/// Type-erased pointer into memory. Guaranteed to be correctly aligned, non-null and safe to modify for a particular type.
pub struct PtrMut<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

/// Type-erased pointer into memory. Guaranteed to be correctly aligned, non-null and safe to move out of for a particular type.
pub struct OwningPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

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
            ///
            /// The lifetime for the returned item must not exceed the lifetime `inner` is valid for
            pub unsafe fn new(inner: NonNull<u8>) -> Self {
                Self(inner, PhantomData)
            }

            pub fn inner(&self) -> NonNull<u8> {
                self.0
            }
        }
    };
}

impl_ptr!(Ptr);
impl<'a> Ptr<'a> {
    /// # Safety
    ///
    /// Another [`PtrMut`] for the same [`Ptr`] must not be created until the first is dropped.
    pub unsafe fn assert_unique(self) -> PtrMut<'a> {
        PtrMut(self.0, PhantomData)
    }

    /// # Safety
    /// Must point to a valid `T`
    pub unsafe fn deref<T>(self) -> &'a T {
        &*self.0.as_ptr().cast()
    }
}
impl_ptr!(PtrMut);
impl<'a> PtrMut<'a> {
    /// Transforms this [`PtrMut`] into an [`OwningPtr`]
    ///
    /// # Safety
    /// Must have right to drop or move out of [`PtrMut`], and current [`PtrMut`] should not be accessed again unless it's written to again.
    pub unsafe fn promote(self) -> OwningPtr<'a> {
        OwningPtr(self.0, PhantomData)
    }

    /// Transforms this [`PtrMut<T>`] into a `&mut T` with the same lifetime
    ///
    /// # Safety
    /// Must point to a valid `T`
    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        &mut *self.inner().as_ptr().cast()
    }
}
impl_ptr!(OwningPtr);
impl<'a> OwningPtr<'a> {
    pub fn make<T, F: FnOnce(OwningPtr<'_>) -> R, R>(val: T, f: F) -> R {
        let mut temp = MaybeUninit::new(val);
        let ptr = unsafe { NonNull::new_unchecked(temp.as_mut_ptr().cast::<u8>()) };
        f(Self(ptr, PhantomData))
    }

    //// Consumes the [`OwningPtr`] to obtain ownership of the underlying data of type `T`.
    ///
    /// # Safety
    /// Must point to a valid `T`.
    pub unsafe fn read<T>(self) -> T {
        self.inner().as_ptr().cast::<T>().read()
    }
}

pub(crate) trait UnsafeCellDeref<'a, T> {
    unsafe fn deref_mut(self) -> &'a mut T;
    unsafe fn deref(self) -> &'a T;
    unsafe fn read(self) -> T
    where
        Self: Copy;
}
impl<'a, T> UnsafeCellDeref<'a, T> for &'a UnsafeCell<T> {
    unsafe fn deref_mut(self) -> &'a mut T {
        &mut *self.get()
    }
    unsafe fn deref(self) -> &'a T {
        &*self.get()
    }

    unsafe fn read(self) -> T
    where
        Self: Copy,
    {
        self.get().read()
    }
}
