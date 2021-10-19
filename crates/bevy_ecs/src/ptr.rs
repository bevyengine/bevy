use std::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

#[derive(Copy, Clone)]
pub struct Ptr<'a>(NonNull<u8>, PhantomData<&'a u8>);
#[derive(Copy, Clone)]
pub struct PtrMut<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

pub struct OwningPtr<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

macro_rules! impl_ptr {
    ($ptr:ident) => {
        impl $ptr<'_> {
            /// Safety: the offset cannot make the existing ptr null, or take it out of bounds for it's allocation.
            pub unsafe fn offset(self, count: isize) -> Self {
                Self(
                    NonNull::new_unchecked(self.0.as_ptr().offset(count)),
                    PhantomData,
                )
            }

            /// Safety: the offset cannot make the existing ptr null, or take it out of bounds for it's allocation.
            pub unsafe fn add(self, count: usize) -> Self {
                Self(
                    NonNull::new_unchecked(self.0.as_ptr().add(count)),
                    PhantomData,
                )
            }

            pub unsafe fn new(inner: NonNull<u8>) -> Self {
                Self(inner, PhantomData)
            }

            pub unsafe fn inner_nonnull(self) -> NonNull<u8> {
                self.0
            }
        }
    };
}

impl_ptr!(Ptr);
impl<'a> Ptr<'a> {
    pub unsafe fn inner(self) -> *const u8 {
        self.0.as_ptr() as *const _
    }

    pub unsafe fn deref<T>(self) -> &'a T {
        &*self.inner().cast()
    }
}
impl_ptr!(PtrMut);
impl<'a> PtrMut<'a> {
    pub unsafe fn inner(self) -> *mut u8 {
        self.0.as_ptr()
    }

    pub unsafe fn promote(self) -> OwningPtr<'a> {
        OwningPtr(self.0, PhantomData)
    }

    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        &mut *self.inner().cast()
    }
}
impl_ptr!(OwningPtr);
impl<'a> OwningPtr<'a> {
    pub unsafe fn inner(self) -> *mut u8 {
        self.0.as_ptr()
    }

    pub fn make<T, F: FnOnce(OwningPtr<'_>) -> R, R>(val: T, f: F) -> R {
        let temp = MaybeUninit::new(val);
        let ptr = unsafe { NonNull::new_unchecked(temp.as_mut_ptr().cast::<u8>()) };
        f(Self(ptr, PhantomData))
    }

    pub unsafe fn read<T>(self) -> T {
        self.inner().cast::<T>().read()
    }

    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        &mut *self.inner().cast()
    }
}
