//! A reimplementation of the currently unstable [`std::cell::SyncUnsafeCell`]
//!
//! [`std::cell::SyncUnsafeCell`]: https://doc.rust-lang.org/nightly/std/cell/struct.SyncUnsafeCell.html

pub use core::cell::UnsafeCell;
use core::ptr;

/// [`UnsafeCell`], but [`Sync`].
///
/// See [tracking issue](https://github.com/rust-lang/rust/issues/95439) for upcoming native impl,
/// which should replace this one entirely (except `from_mut`).
///
/// This is just an `UnsafeCell`, except it implements `Sync`
/// if `T` implements `Sync`.
///
/// `UnsafeCell` doesn't implement `Sync`, to prevent accidental misuse.
/// You can use `SyncUnsafeCell` instead of `UnsafeCell` to allow it to be
/// shared between threads, if that's intentional.
/// Providing proper synchronization is still the task of the user,
/// making this type just as unsafe to use.
///
/// See [`UnsafeCell`] for details.
#[repr(transparent)]
pub struct SyncUnsafeCell<T: ?Sized> {
    value: UnsafeCell<T>,
}

// SAFETY: `T` is Sync, caller is responsible for upholding rust safety rules
unsafe impl<T: ?Sized + Sync> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    /// Constructs a new instance of `SyncUnsafeCell` which will wrap the specified value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Unwraps the value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> SyncUnsafeCell<T> {
    /// Gets a mutable pointer to the wrapped value.
    ///
    /// This can be cast to a pointer of any kind.
    /// Ensure that the access is unique (no active references, mutable or not)
    /// when casting to `&mut T`, and ensure that there are no mutations
    /// or mutable aliases going on when casting to `&T`
    #[inline]
    pub const fn get(&self) -> *mut T {
        self.value.get()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// This call borrows the `SyncUnsafeCell` mutably (at compile-time) which
    /// guarantees that we possess the only reference.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Gets a mutable pointer to the wrapped value.
    ///
    /// See [`UnsafeCell::get`] for details.
    #[inline]
    pub const fn raw_get(this: *const Self) -> *mut T {
        // We can just cast the pointer from `SyncUnsafeCell<T>` to `T` because
        // of #[repr(transparent)] on both SyncUnsafeCell and UnsafeCell.
        // See UnsafeCell::raw_get.
        (this as *const T).cast_mut()
    }

    #[inline]
    /// Returns a `&mut SyncUnsafeCell<T>` from a `&mut T`.
    pub fn from_mut(t: &mut T) -> &mut SyncUnsafeCell<T> {
        let ptr = ptr::from_mut(t) as *mut SyncUnsafeCell<T>;
        // SAFETY: `ptr` must be safe to mutably dereference, since it was originally
        // obtained from a mutable reference. `SyncUnsafeCell` has the same representation
        // as the original type `T`, since the former is annotated with #[repr(transparent)].
        unsafe { &mut *ptr }
    }
}

impl<T> SyncUnsafeCell<[T]> {
    /// Returns a `&[SyncUnsafeCell<T>]` from a `&SyncUnsafeCell<[T]>`.
    /// # Examples
    ///
    /// ```
    /// # use bevy_utils::syncunsafecell::SyncUnsafeCell;
    ///
    /// let slice: &mut [i32] = &mut [1, 2, 3];
    /// let cell_slice: &SyncUnsafeCell<[i32]> = SyncUnsafeCell::from_mut(slice);
    /// let slice_cell: &[SyncUnsafeCell<i32>] = cell_slice.as_slice_of_cells();
    ///
    /// assert_eq!(slice_cell.len(), 3);
    /// ```
    pub fn as_slice_of_cells(&self) -> &[SyncUnsafeCell<T>] {
        let self_ptr: *const SyncUnsafeCell<[T]> = ptr::from_ref(self);
        let slice_ptr = self_ptr as *const [SyncUnsafeCell<T>];
        // SAFETY: `UnsafeCell<T>` and `SyncUnsafeCell<T>` have #[repr(transparent)]
        // therefore:
        // - `SyncUnsafeCell<T>` has the same layout as `T`
        // - `SyncUnsafeCell<[T]>` has the same layout as `[T]`
        // - `SyncUnsafeCell<[T]>` has the same layout as `[SyncUnsafeCell<T>]`
        unsafe { &*slice_ptr }
    }
}

impl<T: Default> Default for SyncUnsafeCell<T> {
    /// Creates an `SyncUnsafeCell`, with the `Default` value for T.
    fn default() -> SyncUnsafeCell<T> {
        SyncUnsafeCell::new(Default::default())
    }
}

impl<T> From<T> for SyncUnsafeCell<T> {
    /// Creates a new `SyncUnsafeCell<T>` containing the given value.
    fn from(t: T) -> SyncUnsafeCell<T> {
        SyncUnsafeCell::new(t)
    }
}
