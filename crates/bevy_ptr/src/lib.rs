#![doc = include_str!("../README.md")]
#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![expect(unsafe_code, reason = "Raw pointers are inherently unsafe.")]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

use core::{
    cell::UnsafeCell,
    fmt::{self, Debug, Formatter, Pointer},
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

/// Used as a type argument to [`Ptr`], [`PtrMut`], [`OwningPtr`], and [`MovingPtr`] to specify that the pointer is guaranteed
/// to be [aligned].
///
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[derive(Debug, Copy, Clone)]
pub struct Aligned;

/// Used as a type argument to [`Ptr`], [`PtrMut`], [`OwningPtr`], and [`MovingPtr`] to specify that the pointer may not [aligned].
///
/// [aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[derive(Debug, Copy, Clone)]
pub struct Unaligned;

/// Trait that is only implemented for [`Aligned`] and [`Unaligned`] to work around the lack of ability
/// to have const generics of an enum.
pub trait IsAligned: sealed::Sealed {
    /// Reads the value pointed to by `ptr`.
    ///
    /// # Safety
    ///  - `ptr` must be valid for reads.
    ///  - `ptr` must point to a valid instance of type `T`
    ///  - If this type is [`Aligned`], then `ptr` must be [properly aligned] for type `T`.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[doc(hidden)]
    unsafe fn read_ptr<T>(ptr: *const T) -> T;

    /// Copies `count * size_of::<T>()` bytes from `src` to `dst`. The source
    /// and destination must *not* overlap.
    ///
    /// # Safety
    ///  - `src` must be valid for reads of `count * size_of::<T>()` bytes.
    ///  - `dst` must be valid for writes of `count * size_of::<T>()` bytes.
    ///  - The region of memory beginning at `src` with a size of `count *
    ///    size_of::<T>()` bytes must *not* overlap with the region of memory
    ///    beginning at `dst` with the same size.
    ///  - If this type is [`Aligned`], then both `src` and `dst` must properly
    ///    be aligned for values of type `T`.
    #[doc(hidden)]
    unsafe fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize);

    /// Reads the value pointed to by `ptr`.
    ///
    /// # Safety
    ///  - `ptr` must be valid for reads and writes.
    ///  - `ptr` must point to a valid instance of type `T`
    ///  - If this type is [`Aligned`], then `ptr` must be [properly aligned] for type `T`.
    ///  - The value pointed to by `ptr` must be valid for dropping.
    ///  - While `drop_in_place` is executing, the only way to access parts of `ptr` is through
    ///    the `&mut Self` supplied to it's `Drop::drop` impl.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[doc(hidden)]
    unsafe fn drop_in_place<T>(ptr: *mut T);
}

impl IsAligned for Aligned {
    #[inline]
    unsafe fn read_ptr<T>(ptr: *const T) -> T {
        // SAFETY:
        //  - The caller is required to ensure that `src` must be valid for reads.
        //  - The caller is required to ensure that `src` points to a valid instance of type `T`.
        //  - This type is `Aligned` so the caller must ensure that `src` is properly aligned for type `T`.
        unsafe { ptr.read() }
    }

    #[inline]
    unsafe fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize) {
        // SAFETY:
        //  - The caller is required to ensure that `src` must be valid for reads.
        //  - The caller is required to ensure that `dst` must be valid for writes.
        //  - The caller is required to ensure that `src` and `dst` are aligned.
        //  - The caller is required to ensure that the memory region covered by `src`
        //    and `dst`, fitting up to `count` elements do not overlap.
        unsafe {
            ptr::copy_nonoverlapping(src, dst, count);
        }
    }

    #[inline]
    unsafe fn drop_in_place<T>(ptr: *mut T) {
        // SAFETY:
        //  - The caller is required to ensure that `ptr` must be valid for reads and writes.
        //  - The caller is required to ensure that `ptr` points to a valid instance of type `T`.
        //  - This type is `Aligned` so the caller must ensure that `ptr` is properly aligned for type `T`.
        //  - The caller is required to ensure that `ptr` points must be valid for dropping.
        //  - The caller is required to ensure that the value `ptr` points must not be used after this function
        //    call.
        unsafe {
            ptr::drop_in_place(ptr);
        }
    }
}

impl IsAligned for Unaligned {
    #[inline]
    unsafe fn read_ptr<T>(ptr: *const T) -> T {
        // SAFETY:
        //  - The caller is required to ensure that `src` must be valid for reads.
        //  - The caller is required to ensure that `src` points to a valid instance of type `T`.
        unsafe { ptr.read_unaligned() }
    }

    #[inline]
    unsafe fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize) {
        // SAFETY:
        //  - The caller is required to ensure that `src` must be valid for reads.
        //  - The caller is required to ensure that `dst` must be valid for writes.
        //  - This is doing a byte-wise copy. `src` and `dst` are always guaranteed to be
        //    aligned.
        //  - The caller is required to ensure that the memory region covered by `src`
        //    and `dst`, fitting up to `count` elements do not overlap.
        unsafe {
            ptr::copy_nonoverlapping::<u8>(
                src.cast::<u8>(),
                dst.cast::<u8>(),
                count * size_of::<T>(),
            );
        }
    }

    #[inline]
    unsafe fn drop_in_place<T>(ptr: *mut T) {
        // SAFETY:
        //  - The caller is required to ensure that `ptr` must be valid for reads and writes.
        //  - The caller is required to ensure that `ptr` points to a valid instance of type `T`.
        //  - This type is not `Aligned` so the caller does not need to ensure that `ptr` is properly aligned for type `T`.
        //  - The caller is required to ensure that `ptr` points must be valid for dropping.
        //  - The caller is required to ensure that the value `ptr` points must not be used after this function
        //    call.
        unsafe {
            drop(ptr.read_unaligned());
        }
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Aligned {}
    impl Sealed for super::Unaligned {}
}

/// A newtype around [`NonNull`] that only allows conversion to read-only borrows or pointers.
///
/// This type can be thought of as the `*const T` to [`NonNull<T>`]'s `*mut T`.
#[repr(transparent)]
pub struct ConstNonNull<T: ?Sized>(NonNull<T>);

impl<T: ?Sized> ConstNonNull<T> {
    /// Creates a new `ConstNonNull` if `ptr` is non-null.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ptr::ConstNonNull;
    ///
    /// let x = 0u32;
    /// let ptr = ConstNonNull::<u32>::new(&x as *const _).expect("ptr is null!");
    ///
    /// if let Some(ptr) = ConstNonNull::<u32>::new(core::ptr::null()) {
    ///     unreachable!();
    /// }
    /// ```
    pub fn new(ptr: *const T) -> Option<Self> {
        NonNull::new(ptr.cast_mut()).map(Self)
    }

    /// Creates a new `ConstNonNull`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ptr::ConstNonNull;
    ///
    /// let x = 0u32;
    /// let ptr = unsafe { ConstNonNull::new_unchecked(&x as *const _) };
    /// ```
    ///
    /// *Incorrect* usage of this function:
    ///
    /// ```rust,no_run
    /// use bevy_ptr::ConstNonNull;
    ///
    /// // NEVER DO THAT!!! This is undefined behavior. ⚠️
    /// let ptr = unsafe { ConstNonNull::<u32>::new_unchecked(core::ptr::null()) };
    /// ```
    pub const unsafe fn new_unchecked(ptr: *const T) -> Self {
        // SAFETY: This function's safety invariants are identical to `NonNull::new_unchecked`
        // The caller must satisfy all of them.
        unsafe { Self(NonNull::new_unchecked(ptr.cast_mut())) }
    }

    /// Returns a shared reference to the value.
    ///
    /// # Safety
    ///
    /// When calling this method, you have to ensure that all of the following is true:
    ///
    /// * The pointer must be [properly aligned].
    ///
    /// * It must be "dereferenceable" in the sense defined in [the module documentation].
    ///
    /// * The pointer must point to an initialized instance of `T`.
    ///
    /// * You must enforce Rust's aliasing rules, since the returned lifetime `'a` is
    ///   arbitrarily chosen and does not necessarily reflect the actual lifetime of the data.
    ///   In particular, while this reference exists, the memory the pointer points to must
    ///   not get mutated (except inside `UnsafeCell`).
    ///
    /// This applies even if the result of this method is unused!
    /// (The part about being initialized is not yet fully decided, but until
    /// it is, the only safe approach is to ensure that they are indeed initialized.)
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ptr::ConstNonNull;
    ///
    /// let mut x = 0u32;
    /// let ptr = ConstNonNull::new(&mut x as *mut _).expect("ptr is null!");
    ///
    /// let ref_x = unsafe { ptr.as_ref() };
    /// println!("{ref_x}");
    /// ```
    ///
    /// [the module documentation]: core::ptr#safety
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        // SAFETY: This function's safety invariants are identical to `NonNull::as_ref`
        // The caller must satisfy all of them.
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized> From<NonNull<T>> for ConstNonNull<T> {
    fn from(value: NonNull<T>) -> ConstNonNull<T> {
        ConstNonNull(value)
    }
}

impl<'a, T: ?Sized> From<&'a T> for ConstNonNull<T> {
    fn from(value: &'a T) -> ConstNonNull<T> {
        ConstNonNull(NonNull::from(value))
    }
}

impl<'a, T: ?Sized> From<&'a mut T> for ConstNonNull<T> {
    fn from(value: &'a mut T) -> ConstNonNull<T> {
        ConstNonNull(NonNull::from(value))
    }
}

/// Type-erased borrow of some unknown type chosen when constructing this type.
///
/// This type tries to act "borrow-like" which means that:
/// - It should be considered immutable: its target must not be changed while this pointer is alive.
/// - It must always point to a valid value of whatever the pointee type is.
/// - The lifetime `'a` accurately represents how long the pointer is valid for.
/// - If `A` is [`Aligned`], the pointer must always be [properly aligned] for the unknown pointee type.
///
/// It may be helpful to think of this type as similar to `&'a dyn Any` but without
/// the metadata and able to point to data that does not correspond to a Rust type.
///
/// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Ptr<'a, A: IsAligned = Aligned>(NonNull<u8>, PhantomData<(&'a u8, A)>);

/// Type-erased mutable borrow of some unknown type chosen when constructing this type.
///
/// This type tries to act "borrow-like" which means that:
/// - Pointer is considered exclusive and mutable. It cannot be cloned as this would lead to
///   aliased mutability.
/// - It must always point to a valid value of whatever the pointee type is.
/// - The lifetime `'a` accurately represents how long the pointer is valid for.
/// - If `A` is [`Aligned`], the pointer must always be [properly aligned] for the unknown pointee type.
///
/// It may be helpful to think of this type as similar to `&'a mut dyn Any` but without
/// the metadata and able to point to data that does not correspond to a Rust type.
///
/// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
#[repr(transparent)]
pub struct PtrMut<'a, A: IsAligned = Aligned>(NonNull<u8>, PhantomData<(&'a mut u8, A)>);

/// Type-erased [`Box`]-like pointer to some unknown type chosen when constructing this type.
///
/// Conceptually represents ownership of whatever data is being pointed to and so is
/// responsible for calling its `Drop` impl. This pointer is _not_ responsible for freeing
/// the memory pointed to by this pointer as it may be pointing to an element in a `Vec` or
/// to a local in a function etc.
///
/// This type tries to act "borrow-like" which means that:
/// - Pointer should be considered exclusive and mutable. It cannot be cloned as this would lead
///   to aliased mutability and potentially use after free bugs.
/// - It must always point to a valid value of whatever the pointee type is.
/// - The lifetime `'a` accurately represents how long the pointer is valid for.
/// - If `A` is [`Aligned`], the pointer must always be [properly aligned] for the unknown pointee type.
///
/// It may be helpful to think of this type as similar to `&'a mut ManuallyDrop<dyn Any>` but
/// without the metadata and able to point to data that does not correspond to a Rust type.
///
/// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
/// [`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
#[repr(transparent)]
pub struct OwningPtr<'a, A: IsAligned = Aligned>(NonNull<u8>, PhantomData<(&'a mut u8, A)>);

/// A [`Box`]-like pointer for moving a value to a new memory location without needing to pass by
/// value.
///
/// Conceptually represents ownership of whatever data is being pointed to and will call its
/// [`Drop`] impl upon being dropped. This pointer is _not_ responsible for freeing
/// the memory pointed to by this pointer as it may be pointing to an element in a `Vec` or
/// to a local in a function etc.
///
/// This type tries to act "borrow-like" which means that:
/// - Pointer should be considered exclusive and mutable. It cannot be cloned as this would lead
///   to aliased mutability and potentially use after free bugs.
/// - It must always point to a valid value of whatever the pointee type is.
/// - The lifetime `'a` accurately represents how long the pointer is valid for.
/// - It does not support pointer arithmetic in any way.
/// - If `A` is [`Aligned`], the pointer must always be [properly aligned] for the type `T`.
///
/// A value can be deconstructed into its fields via [`deconstruct_moving_ptr`], see it's documentation
/// for an example on how to use it.
///
/// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
/// [`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
#[repr(transparent)]
pub struct MovingPtr<'a, T, A: IsAligned = Aligned>(NonNull<T>, PhantomData<(&'a mut T, A)>);

macro_rules! impl_ptr {
    ($ptr:ident) => {
        impl<'a> $ptr<'a, Aligned> {
            /// Removes the alignment requirement of this pointer
            pub fn to_unaligned(self) -> $ptr<'a, Unaligned> {
                $ptr(self.0, PhantomData)
            }
        }

        impl<'a, A: IsAligned> From<$ptr<'a, A>> for NonNull<u8> {
            fn from(ptr: $ptr<'a, A>) -> Self {
                ptr.0
            }
        }

        impl<A: IsAligned> $ptr<'_, A> {
            /// Calculates the offset from a pointer.
            /// As the pointer is type-erased, there is no size information available. The provided
            /// `count` parameter is in raw bytes.
            ///
            /// *See also: [`ptr::offset`][ptr_offset]*
            ///
            /// # Safety
            /// - The offset cannot make the existing ptr null, or take it out of bounds for its allocation.
            /// - If the `A` type parameter is [`Aligned`] then the offset must not make the resulting pointer
            ///   be unaligned for the pointee type.
            /// - The value pointed by the resulting pointer must outlive the lifetime of this pointer.
            ///
            /// [ptr_offset]: https://doc.rust-lang.org/std/primitive.pointer.html#method.offset
            #[inline]
            pub unsafe fn byte_offset(self, count: isize) -> Self {
                Self(
                    // SAFETY: The caller upholds safety for `offset` and ensures the result is not null.
                    unsafe { NonNull::new_unchecked(self.as_ptr().offset(count)) },
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
            /// - The offset cannot make the existing ptr null, or take it out of bounds for its allocation.
            /// - If the `A` type parameter is [`Aligned`] then the offset must not make the resulting pointer
            ///   be unaligned for the pointee type.
            /// - The value pointed by the resulting pointer must outlive the lifetime of this pointer.
            ///
            /// [ptr_add]: https://doc.rust-lang.org/std/primitive.pointer.html#method.add
            #[inline]
            pub unsafe fn byte_add(self, count: usize) -> Self {
                Self(
                    // SAFETY: The caller upholds safety for `add` and ensures the result is not null.
                    unsafe { NonNull::new_unchecked(self.as_ptr().add(count)) },
                    PhantomData,
                )
            }
        }

        impl<A: IsAligned> Pointer for $ptr<'_, A> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                Pointer::fmt(&self.0, f)
            }
        }

        impl Debug for $ptr<'_, Aligned> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}<Aligned>({:?})", stringify!($ptr), self.0)
            }
        }

        impl Debug for $ptr<'_, Unaligned> {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}<Unaligned>({:?})", stringify!($ptr), self.0)
            }
        }
    };
}

impl_ptr!(Ptr);
impl_ptr!(PtrMut);
impl_ptr!(OwningPtr);

impl<'a, T> MovingPtr<'a, T, Aligned> {
    /// Removes the alignment requirement of this pointer
    #[inline]
    pub fn to_unaligned(self) -> MovingPtr<'a, T, Unaligned> {
        let value = MovingPtr(self.0, PhantomData);
        mem::forget(self);
        value
    }

    /// Creates a [`MovingPtr`] from a provided value of type `T`.
    ///
    /// For a safer alternative, it is strongly advised to use [`move_as_ptr`] where possible.
    ///
    /// # Safety
    /// - `value` must store a properly initialized value of type `T`.
    /// - Once the returned [`MovingPtr`] has been used, `value` must be treated as
    ///   it were uninitialized unless it was explicitly leaked via [`core::mem::forget`].
    #[inline]
    pub unsafe fn from_value(value: &'a mut MaybeUninit<T>) -> Self {
        // SAFETY:
        // - MaybeUninit<T> has the same memory layout as T
        // - The caller guarantees that `value` must point to a valid instance of type `T`.
        MovingPtr(NonNull::from(value).cast::<T>(), PhantomData)
    }
}

impl<'a, T, A: IsAligned> MovingPtr<'a, T, A> {
    /// Creates a new instance from a raw pointer.
    ///
    /// For a safer alternative, it is strongly advised to use [`move_as_ptr`] where possible.
    ///
    /// # Safety
    /// - `inner` must point to valid value of `T`.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be be [properly aligned] for `T`.
    /// - `inner` must have correct provenance to allow read and writes of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`MovingPtr`] will stay valid and nothing
    ///   else can read or mutate the pointee while this [`MovingPtr`] is live.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn new(inner: NonNull<T>) -> Self {
        Self(inner, PhantomData)
    }

    /// Partially moves out some fields inside of `self`.
    ///
    /// The partially returned value is returned back pointing to [`MaybeUninit<T>`].
    ///
    /// While calling this function is safe, care must be taken with the returned `MovingPtr` as it
    /// points to a value that may no longer be completely valid.
    ///
    /// # Example
    ///
    /// ```
    /// use core::mem::{offset_of, MaybeUninit};
    /// use bevy_ptr::{MovingPtr, move_as_ptr};
    /// # use bevy_ptr::Unaligned;
    /// # struct FieldAType(usize);
    /// # struct FieldBType(usize);
    /// # struct FieldCType(usize);
    /// # fn insert<T>(_ptr: MovingPtr<'_, T, Unaligned>) {}
    ///
    /// struct Parent {
    ///   field_a: FieldAType,
    ///   field_b: FieldBType,
    ///   field_c: FieldCType,
    /// }
    ///
    /// # let parent = Parent {
    /// #   field_a: FieldAType(0),
    /// #   field_b: FieldBType(0),
    /// #   field_c: FieldCType(0),
    /// # };
    ///
    /// // Converts `parent` into a `MovingPtr`
    /// move_as_ptr!(parent);
    ///
    /// // SAFETY:
    /// // - `field_a` and `field_b` are both unique.
    /// let (partial_parent, ()) = MovingPtr::partial_move(parent, |parent_ptr| unsafe {
    ///   bevy_ptr::deconstruct_moving_ptr!(parent_ptr => {
    ///     field_a,
    ///     field_b,
    ///   });
    ///   
    ///   insert(field_a);
    ///   insert(field_b);
    /// });
    ///
    /// // Move the rest of fields out of the parent.
    /// // SAFETY:
    /// // - `field_c` is by itself unique and does not conflict with the previous accesses
    /// //   inside `partial_move`.
    /// unsafe {
    ///    bevy_ptr::deconstruct_moving_ptr!(partial_parent: MaybeUninit => {
    ///       field_c,
    ///    });
    ///
    ///    insert(field_c);
    /// }
    /// ```
    ///
    /// [`forget`]: core::mem::forget
    #[inline]
    pub fn partial_move<R>(
        self,
        f: impl FnOnce(MovingPtr<'_, T, A>) -> R,
    ) -> (MovingPtr<'a, MaybeUninit<T>, A>, R) {
        let partial_ptr = self.0;
        let ret = f(self);
        (
            MovingPtr(partial_ptr.cast::<MaybeUninit<T>>(), PhantomData),
            ret,
        )
    }

    /// Reads the value pointed to by this pointer.
    #[inline]
    pub fn read(self) -> T {
        // SAFETY:
        //  - `self.0` must be valid for reads as this type owns the value it points to.
        //  - `self.0` must always point to a valid instance of type `T`
        //  - If `A` is [`Aligned`], then `ptr` must be properly aligned for type `T`.
        let value = unsafe { A::read_ptr(self.0.as_ptr()) };
        mem::forget(self);
        value
    }

    /// Writes the value pointed to by this pointer to a provided location.
    ///
    /// This does *not* drop the value stored at `dst` and it's the caller's responsibility
    /// to ensure that it's properly dropped.
    ///
    /// # Safety
    ///  - `dst` must be valid for writes.
    ///  - If the `A` type parameter is [`Aligned`] then `dst` must be [properly aligned] for `T`.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn write_to(self, dst: *mut T) {
        let src = self.0.as_ptr();
        mem::forget(self);
        // SAFETY:
        //  - `src` must be valid for reads as this pointer is considered to own the value it points to.
        //  - The caller is required to ensure that `dst` must be valid for writes.
        //  - As `A` is `Aligned`, the caller is required to ensure that `dst` is aligned and `src` must
        //    be aligned by the type's invariants.
        unsafe { A::copy_nonoverlapping(src, dst, 1) };
    }

    /// Writes the value pointed to by this pointer into `dst`.
    ///
    /// The value previously stored at `dst` will be dropped.
    #[inline]
    pub fn assign_to(self, dst: &mut T) {
        // SAFETY:
        // - `dst` is a mutable borrow, it must point to a valid instance of `T`.
        // - `dst` is a mutable borrow, it must point to value that is valid for dropping.
        // - `dst` is a mutable borrow, it must not alias any other access.
        unsafe {
            ptr::drop_in_place(dst);
        }
        // SAFETY:
        // - `dst` is a mutable borrow, it must be valid for writes.
        // - `dst` is a mutable borrow, it must always be aligned.
        unsafe {
            self.write_to(dst);
        }
    }

    /// Creates a [`MovingPtr`] for a specific field within `self`.
    ///
    /// This function is explicitly made for deconstructive moves.
    ///
    /// The correct `byte_offset` for a field can be obtained via [`core::mem::offset_of`].
    ///
    /// The returned value will always be considered unaligned as `repr(packed)` types may result in
    /// unaligned fields. The pointer is convertible back into an aligned one using the [`TryFrom`] impl.
    ///
    /// # Safety
    ///  - `f` must return a non-null pointer to a valid field inside `T`
    ///  - `self` should not be accessed or dropped as if it were a complete value after this function returns.
    ///    Other fields that have not been moved out of may still be accessed or dropped separately.
    ///  - This function cannot alias the field with any other access, including other calls to [`move_field`]
    ///    for the same field, without first calling [`forget`] on it first.
    ///
    /// A result of the above invariants means that any operation that could cause `self` to be dropped while
    /// the pointers to the fields are held will result in undefined behavior. This requires exctra caution
    /// around code that may panic. See the example below for an example of how to safely use this function.
    ///
    /// # Example
    ///
    /// ```
    /// use core::mem::offset_of;
    /// use bevy_ptr::{MovingPtr, move_as_ptr};
    /// # use bevy_ptr::Unaligned;
    /// # struct FieldAType(usize);
    /// # struct FieldBType(usize);
    /// # struct FieldCType(usize);
    /// # fn insert<T>(_ptr: MovingPtr<'_, T, Unaligned>) {}
    ///
    /// struct Parent {
    ///   field_a: FieldAType,
    ///   field_b: FieldBType,
    ///   field_c: FieldCType,
    /// }
    ///
    /// let parent = Parent {
    ///    field_a: FieldAType(0),
    ///    field_b: FieldBType(0),
    ///    field_c: FieldCType(0),
    /// };
    ///
    /// // Converts `parent` into a `MovingPtr`.
    /// move_as_ptr!(parent);
    ///
    /// unsafe {
    ///    let field_a = parent.move_field(|ptr| &raw mut (*ptr).field_a);
    ///    let field_b = parent.move_field(|ptr| &raw mut (*ptr).field_b);
    ///    let field_c = parent.move_field(|ptr| &raw mut (*ptr).field_c);
    ///    // Each call to insert may panic! Ensure that `parent_ptr` cannot be dropped before
    ///    // calling them!
    ///    core::mem::forget(parent);
    ///    insert(field_a);
    ///    insert(field_b);
    ///    insert(field_c);
    /// }
    /// ```
    ///
    /// [`forget`]: core::mem::forget
    /// [`move_field`]: Self::move_field
    #[inline(always)]
    pub unsafe fn move_field<U>(
        &self,
        f: impl Fn(*mut T) -> *mut U,
    ) -> MovingPtr<'a, U, Unaligned> {
        MovingPtr(
            // SAFETY: The caller must ensure that `U` is the correct type for the field at `byte_offset`.
            unsafe { NonNull::new_unchecked(f(self.0.as_ptr())) },
            PhantomData,
        )
    }
}

impl<'a, T, A: IsAligned> MovingPtr<'a, MaybeUninit<T>, A> {
    /// Creates a [`MovingPtr`] for a specific field within `self`.
    ///
    /// This function is explicitly made for deconstructive moves.
    ///
    /// The correct `byte_offset` for a field can be obtained via [`core::mem::offset_of`].
    ///
    /// The returned value will always be considered unaligned as `repr(packed)` types may result in
    /// unaligned fields. The pointer is convertible back into an aligned one using the [`TryFrom`] impl.
    ///
    /// # Safety
    ///  - `f` must return a non-null pointer to a valid field inside `T`
    #[inline(always)]
    pub unsafe fn move_maybe_uninit_field<U>(
        &self,
        f: impl Fn(*mut T) -> *mut U,
    ) -> MovingPtr<'a, MaybeUninit<U>, Unaligned> {
        let self_ptr = self.0.as_ptr().cast::<T>();
        // SAFETY:
        // - The caller must ensure that `U` is the correct type for the field at `byte_offset` and thus
        //   cannot be null.
        // - `MaybeUninit<T>` is `repr(transparent)` and thus must have the same memory layout as `T``
        let field_ptr = unsafe { NonNull::new_unchecked(f(self_ptr)) };
        MovingPtr(field_ptr.cast::<MaybeUninit<U>>(), PhantomData)
    }
}

impl<'a, T, A: IsAligned> MovingPtr<'a, MaybeUninit<T>, A> {
    /// Creates a [`MovingPtr`] pointing to a valid instance of `T`.
    ///
    /// See also: [`MaybeUninit::assume_init`].
    ///
    /// # Safety
    /// It's up to the caller to ensure that the value pointed to by `self`
    /// is really in an initialized state. Calling this when the content is not yet
    /// fully initialized causes immediate undefined behavior.
    #[inline]
    pub unsafe fn assume_init(self) -> MovingPtr<'a, T, A> {
        let value = MovingPtr(self.0.cast::<T>(), PhantomData);
        mem::forget(self);
        value
    }
}

impl<T, A: IsAligned> Pointer for MovingPtr<'_, T, A> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Pointer::fmt(&self.0, f)
    }
}

impl<T> Debug for MovingPtr<'_, T, Aligned> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "MovingPtr<Aligned>({:?})", self.0)
    }
}

impl<T> Debug for MovingPtr<'_, T, Unaligned> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "MovingPtr<Unaligned>({:?})", self.0)
    }
}

impl<'a, T, A: IsAligned> From<MovingPtr<'a, T, A>> for OwningPtr<'a, A> {
    #[inline]
    fn from(value: MovingPtr<'a, T, A>) -> Self {
        // SAFETY:
        // - `value.0` must always point to valid value of type `T`.
        // - The type parameter `A` is mirrored from input to output, keeping the same alignment guarantees.
        // - `value.0` by construction must have correct provenance to allow read and writes of type `T`.
        // - The lifetime `'a` is mirrored from input to output, keeping the same lifetime guarantees.
        // - `OwningPtr` maintains the same aliasing invariants as `MovingPtr`.
        let ptr = unsafe { OwningPtr::new(value.0.cast::<u8>()) };
        mem::forget(value);
        ptr
    }
}

impl<'a, T> TryFrom<MovingPtr<'a, T, Unaligned>> for MovingPtr<'a, T, Aligned> {
    type Error = MovingPtr<'a, T, Unaligned>;
    #[inline]
    fn try_from(value: MovingPtr<'a, T, Unaligned>) -> Result<Self, Self::Error> {
        let ptr = value.0;
        if ptr.as_ptr().is_aligned() {
            mem::forget(value);
            Ok(MovingPtr(ptr, PhantomData))
        } else {
            Err(value)
        }
    }
}

impl<T> Deref for MovingPtr<'_, T, Aligned> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        let ptr = self.0.as_ptr().debug_ensure_aligned();
        // SAFETY: This type owns the value it points to and the generic type parameter is `A` so this pointer must be aligned.
        unsafe { &*ptr }
    }
}

impl<T> DerefMut for MovingPtr<'_, T, Aligned> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = self.0.as_ptr().debug_ensure_aligned();
        // SAFETY: This type owns the value it points to and the generic type parameter is `A` so this pointer must be aligned.
        unsafe { &mut *ptr }
    }
}

impl<T, A: IsAligned> Drop for MovingPtr<'_, T, A> {
    fn drop(&mut self) {
        // SAFETY:
        //  - `self.0` must be valid for reads and writes as this pointer type owns the value it points to.
        //  - `self.0` must always point to a valid instance of type `T`
        //  - If `A` is `Aligned`, then `ptr` must be properly aligned for type `T` by construction.
        //  - `self.0` owns the value it points to so it must always be valid for dropping until this pointer is dropped.
        //  - This type owns the value it points to, so it's required to not mutably alias value that it points to.
        unsafe { A::drop_in_place(self.0.as_ptr()) };
    }
}

impl<'a, A: IsAligned> Ptr<'a, A> {
    /// Creates a new instance from a raw pointer.
    ///
    /// # Safety
    /// - `inner` must point to valid value of whatever the pointee type is.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be be [properly aligned]for the pointee type.
    /// - `inner` must have correct provenance to allow reads of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`Ptr`] will stay valid and nothing
    ///   can mutate the pointee while this [`Ptr`] is live except through an [`UnsafeCell`].
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn new(inner: NonNull<u8>) -> Self {
        Self(inner, PhantomData)
    }

    /// Transforms this [`Ptr`] into an [`PtrMut`]
    ///
    /// # Safety
    /// * The data pointed to by this `Ptr` must be valid for writes.
    /// * There must be no active references (mutable or otherwise) to the data underlying this `Ptr`.
    /// * Another [`PtrMut`] for the same [`Ptr`] must not be created until the first is dropped.
    #[inline]
    pub unsafe fn assert_unique(self) -> PtrMut<'a, A> {
        PtrMut(self.0, PhantomData)
    }

    /// Transforms this [`Ptr<T>`] into a `&T` with the same lifetime
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`Ptr`].
    /// - If the type parameter `A` is [`Unaligned`] then this pointer must be be [properly aligned]
    ///   for the pointee type `T`.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn deref<T>(self) -> &'a T {
        let ptr = self.as_ptr().cast::<T>().debug_ensure_aligned();
        // SAFETY: The caller ensures the pointee is of type `T` and the pointer can be dereferenced.
        unsafe { &*ptr }
    }

    /// Gets the underlying pointer, erasing the associated lifetime.
    ///
    /// If possible, it is strongly encouraged to use [`deref`](Self::deref) over this function,
    /// as it retains the lifetime.
    #[inline]
    pub fn as_ptr(self) -> *mut u8 {
        self.0.as_ptr()
    }
}

impl<'a, T: ?Sized> From<&'a T> for Ptr<'a> {
    #[inline]
    fn from(val: &'a T) -> Self {
        // SAFETY: The returned pointer has the same lifetime as the passed reference.
        // Access is immutable.
        unsafe { Self::new(NonNull::from(val).cast()) }
    }
}

impl<'a, A: IsAligned> PtrMut<'a, A> {
    /// Creates a new instance from a raw pointer.
    ///
    /// # Safety
    /// - `inner` must point to valid value of whatever the pointee type is.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be be [properly aligned] for the pointee type.
    /// - `inner` must have correct provenance to allow read and writes of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`PtrMut`] will stay valid and nothing
    ///   else can read or mutate the pointee while this [`PtrMut`] is live.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn new(inner: NonNull<u8>) -> Self {
        Self(inner, PhantomData)
    }

    /// Transforms this [`PtrMut`] into an [`OwningPtr`]
    ///
    /// # Safety
    /// Must have right to drop or move out of [`PtrMut`].
    #[inline]
    pub unsafe fn promote(self) -> OwningPtr<'a, A> {
        OwningPtr(self.0, PhantomData)
    }

    /// Transforms this [`PtrMut<T>`] into a `&mut T` with the same lifetime
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`PtrMut`].
    /// - If the type parameter `A` is [`Unaligned`] then this pointer must be be [properly aligned]
    ///   for the pointee type `T`.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        let ptr = self.as_ptr().cast::<T>().debug_ensure_aligned();
        // SAFETY: The caller ensures the pointee is of type `T` and the pointer can be dereferenced.
        unsafe { &mut *ptr }
    }

    /// Gets the underlying pointer, erasing the associated lifetime.
    ///
    /// If possible, it is strongly encouraged to use [`deref_mut`](Self::deref_mut) over
    /// this function, as it retains the lifetime.
    #[inline]
    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }

    /// Gets a [`PtrMut`] from this with a smaller lifetime.
    #[inline]
    pub fn reborrow(&mut self) -> PtrMut<'_, A> {
        // SAFETY: the ptrmut we're borrowing from is assumed to be valid
        unsafe { PtrMut::new(self.0) }
    }

    /// Gets an immutable reference from this mutable reference
    #[inline]
    pub fn as_ref(&self) -> Ptr<'_, A> {
        // SAFETY: The `PtrMut` type's guarantees about the validity of this pointer are a superset of `Ptr` s guarantees
        unsafe { Ptr::new(self.0) }
    }
}

impl<'a, T: ?Sized> From<&'a mut T> for PtrMut<'a> {
    #[inline]
    fn from(val: &'a mut T) -> Self {
        // SAFETY: The returned pointer has the same lifetime as the passed reference.
        // The reference is mutable, and thus will not alias.
        unsafe { Self::new(NonNull::from(val).cast()) }
    }
}

impl<'a> OwningPtr<'a> {
    /// This exists mostly to reduce compile times;
    /// code is only duplicated per type, rather than per function called.
    ///
    /// # Safety
    ///
    /// Safety constraints of [`PtrMut::promote`] must be upheld.
    unsafe fn make_internal<T>(temp: &mut ManuallyDrop<T>) -> OwningPtr<'_> {
        // SAFETY: The constraints of `promote` are upheld by caller.
        unsafe { PtrMut::from(&mut *temp).promote() }
    }

    /// Consumes a value and creates an [`OwningPtr`] to it while ensuring a double drop does not happen.
    #[inline]
    pub fn make<T, F: FnOnce(OwningPtr<'_>) -> R, R>(val: T, f: F) -> R {
        let mut val = ManuallyDrop::new(val);
        // SAFETY: The value behind the pointer will not get dropped or observed later,
        // so it's safe to promote it to an owning pointer.
        f(unsafe { Self::make_internal(&mut val) })
    }
}

impl<'a, A: IsAligned> OwningPtr<'a, A> {
    /// Creates a new instance from a raw pointer.
    ///
    /// # Safety
    /// - `inner` must point to valid value of whatever the pointee type is.
    /// - If the `A` type parameter is [`Aligned`] then `inner` must be [properly aligned] for the pointee type.
    /// - `inner` must have correct provenance to allow read and writes of the pointee type.
    /// - The lifetime `'a` must be constrained such that this [`OwningPtr`] will stay valid and nothing
    ///   else can read or mutate the pointee while this [`OwningPtr`] is live.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn new(inner: NonNull<u8>) -> Self {
        Self(inner, PhantomData)
    }

    /// Consumes the [`OwningPtr`] to obtain ownership of the underlying data of type `T`.
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`OwningPtr`].
    /// - If the type parameter `A` is [`Unaligned`] then this pointer must be be [properly aligned]
    ///   for the pointee type `T`.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn read<T>(self) -> T {
        let ptr = self.as_ptr().cast::<T>().debug_ensure_aligned();
        // SAFETY: The caller ensure the pointee is of type `T` and uphold safety for `read`.
        unsafe { ptr.read() }
    }

    /// Casts to a concrete type as a [`MovingPtr`].
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`OwningPtr`].
    #[inline]
    pub unsafe fn cast<T>(self) -> MovingPtr<'a, T, A> {
        MovingPtr(self.0.cast::<T>(), PhantomData)
    }

    /// Consumes the [`OwningPtr`] to drop the underlying data of type `T`.
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`OwningPtr`].
    /// - If the type parameter `A` is [`Unaligned`] then this pointer must be be [properly aligned]
    ///   for the pointee type `T`.
    ///
    /// [properly aligned]: https://doc.rust-lang.org/std/ptr/index.html#alignment
    #[inline]
    pub unsafe fn drop_as<T>(self) {
        let ptr = self.as_ptr().cast::<T>().debug_ensure_aligned();
        // SAFETY: The caller ensure the pointee is of type `T` and uphold safety for `drop_in_place`.
        unsafe {
            ptr.drop_in_place();
        }
    }

    /// Gets the underlying pointer, erasing the associated lifetime.
    ///
    /// If possible, it is strongly encouraged to use the other more type-safe functions
    /// over this function.
    #[inline]
    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }

    /// Gets an immutable pointer from this owned pointer.
    #[inline]
    pub fn as_ref(&self) -> Ptr<'_, A> {
        // SAFETY: The `Owning` type's guarantees about the validity of this pointer are a superset of `Ptr` s guarantees
        unsafe { Ptr::new(self.0) }
    }

    /// Gets a mutable pointer from this owned pointer.
    #[inline]
    pub fn as_mut(&mut self) -> PtrMut<'_, A> {
        // SAFETY: The `Owning` type's guarantees about the validity of this pointer are a superset of `Ptr` s guarantees
        unsafe { PtrMut::new(self.0) }
    }
}

impl<'a> OwningPtr<'a, Unaligned> {
    /// Consumes the [`OwningPtr`] to obtain ownership of the underlying data of type `T`.
    ///
    /// # Safety
    /// - `T` must be the erased pointee type for this [`OwningPtr`].
    pub unsafe fn read_unaligned<T>(self) -> T {
        let ptr = self.as_ptr().cast::<T>();
        // SAFETY: The caller ensure the pointee is of type `T` and uphold safety for `read_unaligned`.
        unsafe { ptr.read_unaligned() }
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

        let ptr = self.ptr.as_ptr();
        // SAFETY: `index` is in-bounds so the resulting pointer is valid to dereference.
        unsafe { &*ptr.add(index) }
    }
}

impl<'a, T> Clone for ThinSlicePtr<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for ThinSlicePtr<'a, T> {}

impl<'a, T> From<&'a [T]> for ThinSlicePtr<'a, T> {
    #[inline]
    fn from(slice: &'a [T]) -> Self {
        let ptr = slice.as_ptr().cast_mut();
        Self {
            // SAFETY: a reference can never be null
            ptr: unsafe { NonNull::new_unchecked(ptr.debug_ensure_aligned()) },
            #[cfg(debug_assertions)]
            len: slice.len(),
            _marker: PhantomData,
        }
    }
}

/// Creates a dangling pointer with specified alignment.
/// See [`NonNull::dangling`].
pub const fn dangling_with_align(align: NonZeroUsize) -> NonNull<u8> {
    debug_assert!(align.is_power_of_two(), "Alignment must be power of two.");
    // SAFETY: The pointer will not be null, since it was created
    // from the address of a `NonZero<usize>`.
    // TODO: use https://doc.rust-lang.org/std/ptr/struct.NonNull.html#method.with_addr once stabilized
    unsafe { NonNull::new_unchecked(ptr::null_mut::<u8>().wrapping_add(align.get())) }
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
        // SAFETY: The caller upholds the alias rules.
        unsafe { &mut *self.get() }
    }
    #[inline]
    unsafe fn deref(self) -> &'a T {
        // SAFETY: The caller upholds the alias rules.
        unsafe { &*self.get() }
    }

    #[inline]
    unsafe fn read(self) -> T
    where
        T: Copy,
    {
        // SAFETY: The caller upholds the alias rules.
        unsafe { self.get().read() }
    }
}

trait DebugEnsureAligned {
    fn debug_ensure_aligned(self) -> Self;
}

// Disable this for miri runs as it already checks if pointer to reference
// casts are properly aligned.
#[cfg(all(debug_assertions, not(miri)))]
impl<T: Sized> DebugEnsureAligned for *mut T {
    #[track_caller]
    fn debug_ensure_aligned(self) -> Self {
        let align = align_of::<T>();
        // Implementation shamelessly borrowed from the currently unstable
        // ptr.is_aligned_to.
        //
        // Replace once https://github.com/rust-lang/rust/issues/96284 is stable.
        assert_eq!(
            self as usize & (align - 1),
            0,
            "pointer is not aligned. Address {:p} does not have alignment {} for type {}",
            self,
            align,
            core::any::type_name::<T>()
        );
        self
    }
}

#[cfg(any(not(debug_assertions), miri))]
impl<T: Sized> DebugEnsureAligned for *mut T {
    #[inline(always)]
    fn debug_ensure_aligned(self) -> Self {
        self
    }
}

/// Safely converts a owned value into a [`MovingPtr`] while minimizing the number of stack copies.
///
/// This cannot be used as expression and must be used as a statement. Internally this macro works via variable shadowing.
#[macro_export]
macro_rules! move_as_ptr {
    ($value: ident) => {
        let mut $value = core::mem::MaybeUninit::new($value);
        // SAFETY:
        // - This macro shadows a MaybeUninit value that took ownership of the original value.
        //   it is impossible to refer to the original value, preventing further access after
        //   the `MovingPtr` has been used. `MaybeUninit` also prevents the compiler from
        //   dropping the original value.
        let $value = unsafe { bevy_ptr::MovingPtr::from_value(&mut $value) };
    };
}

/// Deconstructs a [`MovingPtr`] into its individual fields.
///
/// This consumes the [`MovingPtr`] and hands out [`MovingPtr`] wrappers around
/// pointers to each of its fields. The value will *not* be dropped.
///
/// The field move expressions will be executed in the order they're provided to the macro.
/// In the example below, the call to [`assign_to`] for `field_a` will always run before the
/// calls for `field_b` and `field_c`.
///
/// # Safety
/// This macro generates unsafe code and must be set up correctly to avoid undefined behavior.
///  - Each field accessed must be unique, multiple of the same field cannot be listed.
///
/// # Examples
///
/// ## Structs
///
/// ```
/// use core::mem::{offset_of, MaybeUninit};
/// use bevy_ptr::{MovingPtr, move_as_ptr};
/// # use bevy_ptr::Unaligned;
/// # struct FieldAType(usize);
/// # struct FieldBType(usize);
/// # struct FieldCType(usize);
///
/// # pub struct Parent {
/// #  pub field_a: FieldAType,
/// #  pub field_b: FieldBType,
/// #  pub field_c: FieldCType,
/// # }
///
/// let parent = Parent {
///   field_a: FieldAType(11),
///   field_b: FieldBType(22),
///   field_c: FieldCType(33),
/// };
///
/// let mut target_a = FieldAType(101);
/// let mut target_b = FieldBType(102);
/// let mut target_c = FieldCType(103);
///
/// // Converts `parent` into a `MovingPtr`
/// move_as_ptr!(parent);
///
/// // The field names must match the name used in the type definition.
/// // Each one will be a `MovingPtr` of the field's type.
/// unsafe {
///   bevy_ptr::deconstruct_moving_ptr!(parent => {
///      field_a,
///      field_b,
///      field_c,
///   });
///
///   field_a.assign_to(&mut target_a);
///   field_b.assign_to(&mut target_b);
///   field_c.assign_to(&mut target_c);
/// }
///
/// assert_eq!(target_a.0, 11);
/// assert_eq!(target_b.0, 22);
/// assert_eq!(target_c.0, 33);
/// ```
///
/// ## Tuples
///
/// ```
/// use core::mem::{offset_of, MaybeUninit};
/// use bevy_ptr::{MovingPtr, move_as_ptr};
/// # use bevy_ptr::Unaligned;
/// # struct FieldAType(usize);
/// # struct FieldBType(usize);
/// # struct FieldCType(usize);
///
/// # pub struct Parent {
/// #   pub field_a: FieldAType,
/// #  pub field_b: FieldBType,
/// #  pub field_c: FieldCType,
/// # }
///
/// let parent = (
///   FieldAType(11),
///   FieldBType(22),
///   FieldCType(33),
/// );
///
/// let mut target_a = FieldAType(101);
/// let mut target_b = FieldBType(102);
/// let mut target_c = FieldCType(103);
///
/// // Converts `parent` into a `MovingPtr`
/// move_as_ptr!(parent);
///
/// // The field names must match the name used in the type definition.
/// // Each one will be a `MovingPtr` of the field's type.
/// unsafe {
///   bevy_ptr::deconstruct_moving_ptr!(parent => (
///      0 => field_a,
///      1 => field_b,
///      2 => field_c,
///   ));
///
///   field_a.assign_to(&mut target_a);
///   field_b.assign_to(&mut target_b);
///   field_c.assign_to(&mut target_c);
/// }
///
/// assert_eq!(target_a.0, 11);
/// assert_eq!(target_b.0, 22);
/// assert_eq!(target_c.0, 33);
/// ```
///
/// [`assign_to`]: MovingPtr::assign_to
#[macro_export]
macro_rules! deconstruct_moving_ptr {
    ($ptr:ident => {$($field_name:ident,)*}) => {
        $crate::deconstruct_moving_ptr!($ptr => ($($field_name => $field_name,)*))
    };
    ($ptr:ident => ($($field_index:tt => $field_alias:ident,)*)) => {
        $(let $field_alias = $ptr.move_field(|f| &raw mut (*f).$field_index);)*
        core::mem::forget($ptr);
    };
    ($ptr:ident: MaybeUninit => {$($field_name:tt,)*}) => {
        $crate::deconstruct_moving_ptr!($ptr: MaybeUninit => ($($field_name => $field_name,)*))
    };
    ($ptr:ident: MaybeUninit => ($($field_index:tt => $field_alias:ident,)*)) => {
        $(let $field_alias = $ptr.move_maybe_uninit_field(|f| &raw mut (*f).$field_index);)*
        core::mem::forget($ptr);
    };
}
