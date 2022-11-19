use core::alloc::Layout;
use core::borrow::{Borrow, BorrowMut};
use core::marker::PhantomData;
use core::mem::needs_drop;
use core::ops::{Deref, DerefMut};

use core::cmp;
use core::slice::SliceIndex;

use crate::ptr::OwningPtr;

use super::blob_vec::BlobVec;

/// A vector whose internal buffer is aligned to `MAX_SIMD_ALIGNMENT`.
/// Intended to support SIMD use cases.  Aligning the data to `MAX_SIMD_ALIGNMENT`
/// allows for best-case alignment on accesses, which helps performance when using batched
/// queries.
///
/// Used to densely store homogeneous ECS data whose type is known at compile time.
/// Built on `BlobVec`. It is not intended to be a drop-in replacement for Vec at this time.

/*
NOTE: AlignedVec is ONLY implemented in terms of BlobVec because the Allocator API is not stable yet.
Once the Allocator API is stable, one could easily define AlignedVec as being a Vec with an allocator
that provides MAX_SIMD_ALIGNMENT as a guarantee, and remove almost all of the code in this file:

    type AlignedVec<T> = Vec<T,AlignedAllocator>;

As it stands, AlignedVec is a stand-in to provide just enough functionality to work for bevy_ecs.
*/
pub(crate) struct SimdAlignedVec<T> {
    vec: BlobVec,
    _marker: PhantomData<T>,
}

impl<T> Default for SimdAlignedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for SimdAlignedVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlignedVec")
            .field("vec", &self.vec)
            .finish()
    }
}

impl<T> SimdAlignedVec<T> {
    // SAFETY: The pointer points to a valid value of type `T` and it is safe to drop this value.
    unsafe fn drop_ptr(x: OwningPtr<'_>) {
        x.drop_as::<T>();
    }

    pub fn with_capacity(capacity: usize) -> SimdAlignedVec<T> {
        Self {
            // SAFETY:
            // `drop` accurately reflects whether the contents of this Vec need to be dropped, and correctly performs the drop operation.
            vec: unsafe {
                BlobVec::new(
                    Layout::new::<T>(),
                    needs_drop::<T>().then_some(Self::drop_ptr as _),
                    capacity,
                )
            },
            _marker: PhantomData,
        }
    }

    pub fn new() -> SimdAlignedVec<T> {
        Self::with_capacity(0) //Ensure a starting power-of-two capacity (for non-ZSTs)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.len() == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < self.len()
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &<usize as SliceIndex<[T]>>::Output {
        debug_assert!(index < self.len());

        self.vec.get_unchecked(index).deref()
    }

    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < self.len()
    #[inline]
    pub unsafe fn get_unchecked_mut(
        &mut self,
        index: usize,
    ) -> &mut <usize as SliceIndex<[T]>>::Output {
        debug_assert!(index < self.len());

        self.vec.get_unchecked_mut(index).deref_mut()
    }

    //This function attempts to keep the same semantics as Vec's swap_remove function
    pub fn swap_remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("swap_remove index (is {index}) should be < len (is {len})");
        }
        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }

        // SAFETY:
        // The index is guaranteed to be in bounds by this point.
        unsafe { self.vec.swap_remove_and_forget_unchecked(index).read() }
    }

    pub fn push(&mut self, value: T) {
        // SAFETY:
        // value is a valid owned instance of T, therefore it is safe to call push with it
        OwningPtr::make(value, |ptr| unsafe {
            self.vec.push(ptr);
        });
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.vec.reserve_exact(additional);
    }

    // From RawVec soruce code, for compatibility
    const MIN_NON_ZERO_CAP: usize = if core::mem::size_of::<T>() == 1 {
        8
    } else if core::mem::size_of::<T>() <= 1024 {
        4
    } else {
        1
    };

    //This function attempts to keep the same semantics as Vec's reserve function
    pub fn reserve(&mut self, additional: usize) {
        if core::mem::size_of::<T>() == 0 {
            // Since we return a capacity of `usize::MAX` when `elem_size` is
            // 0, getting to here necessarily means the `AlignedVec` is overfull.
            panic!("AlignedVec capacity overflow")
        }

        // Nothing we can really do about these checks, sadly.
        let required_cap = self.vec.len().checked_add(additional);

        if let Some(cap) = required_cap {
            // This guarantees exponential growth. The doubling cannot overflow
            // because `cap <= isize::MAX` and the type of `cap` is `usize`.
            let cap = cmp::max(self.vec.capacity() * 2, cap);
            let cap = cmp::max(Self::MIN_NON_ZERO_CAP, cap);

            self.reserve_exact(cap - self.vec.len());
        } else {
            panic!("AlignedVec capacity overflow")
        }
    }

    pub fn clear(&mut self) {
        self.vec.clear();
    }
}

impl<T> Borrow<[T]> for SimdAlignedVec<T> {
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T> BorrowMut<[T]> for SimdAlignedVec<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> AsRef<[T]> for SimdAlignedVec<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for SimdAlignedVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> Deref for SimdAlignedVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        // SAFETY:
        // The vector represents an array of T with appropriate alignment.
        // The vector is borrowed with an shared reference, guaranteeing only other shared references exist.
        // Therefore, it is safe to provide a shared reference to its contents.
        unsafe {
            std::slice::from_raw_parts(self.vec.get_ptr().as_ptr() as *const T, self.vec.len())
        }
    }
}

impl<T> DerefMut for SimdAlignedVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY:
        // The vector represents an array of T with appropriate alignment.
        // The vector is borrowed with a mutable reference, guaranteeing uniqueness.
        // Therefore, it is safe to provide a mutable reference to its contents.
        unsafe {
            core::slice::from_raw_parts_mut(
                self.vec.get_ptr_mut().as_ptr() as *mut T,
                self.vec.len(),
            )
        }
    }
}

impl<'a, T> IntoIterator for &'a mut SimdAlignedVec<T> {
    type Item = <&'a mut [T] as IntoIterator>::Item;

    type IntoIter = <&'a mut [T] as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut().iter_mut()
    }
}

impl<'a, T> IntoIterator for &'a SimdAlignedVec<T> {
    type Item = <&'a [T] as IntoIterator>::Item;

    type IntoIter = <&'a [T] as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}
