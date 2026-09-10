use core::any::{Any, TypeId};

/// Checks if the current type "is" another type, using a [`TypeId`] equality comparison.
pub trait Is {
    /// Checks if the current type "is" another type, using a [`TypeId`] equality comparison.
    /// This is most useful in the context of generic logic.
    ///
    /// ```
    /// # use bevy_reflect::Is;
    /// # use std::any::Any;
    /// fn greet_if_u32<T: Any>() {
    ///     if T::is::<u32>() {
    ///         println!("Hello");
    ///     }
    /// }
    /// // this will print "Hello"
    /// greet_if_u32::<u32>();
    /// // this will not print "Hello"
    /// greet_if_u32::<String>();
    /// assert!(u32::is::<u32>());
    /// assert!(!usize::is::<u32>());
    /// ```
    fn is<T: Any>() -> bool;
}

impl<A: Any> Is for A {
    #[inline]
    fn is<T: Any>() -> bool {
        TypeId::of::<A>() == TypeId::of::<T>()
    }
}
