macro_rules! impl_non_max_fmt {
    (($($trait:ident),+) for $ty:ident) => {
        $(
            impl std::fmt::$trait for $ty {
                #[inline]
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.get().fmt(f)
                }
            }
        )+
    }
}

macro_rules! impl_non_max {
    ($nonmax:ident, $nonzero:ty, $repr:ty, $test:ident) => {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        #[repr(transparent)]
        pub struct $nonmax($nonzero);

        impl $nonmax {
            /// Creates a non-max if `n` is not the maximum value.
            #[inline]
            #[allow(clippy::manual_map)]
            pub const fn new(n: $repr) -> Option<Self> {
                if let Some(n) = <$nonzero>::new(!n) {
                    Some(Self(n))
                } else {
                    None
                }
            }

            /// Creates a non-max without checking the value.
            ///
            /// # Safety
            /// `n` must not be equal to T::MAX.
            #[inline]
            pub const unsafe fn new_unchecked(n: $repr) -> Self {
                Self(<$nonzero>::new_unchecked(!n))
            }

            /// Returns the value as a primitive type.
            ///
            /// # Note
            /// This function is not free. Consider storing the result
            /// into a variable instead of calling `get()` multiple times.
            #[inline]
            pub const fn get(self) -> $repr {
                !self.0.get()
            }
        }

        impl_non_max_fmt! {
            (Debug, Display, Binary, Octal, LowerHex, UpperHex) for $nonmax
        }

        #[cfg(test)]
        mod $test {
            use super::*;

            #[test]
            fn test() {
                assert!($nonmax::new(<$repr>::MAX).is_none());
                assert_eq!($nonmax::new(0).unwrap().get(), 0);
                assert_eq!($nonmax::new(1).unwrap().get(), 1);

                // SAFE: `0` != <$repr>::MAX
                unsafe {
                    assert_eq!($nonmax::new_unchecked(0).get(), 0);
                }

                assert_eq!(
                    std::mem::size_of::<$nonmax>(),
                    std::mem::size_of::<Option<$nonmax>>()
                );
            }
        }
    };
}

impl_non_max!(
    NonMaxUsize,
    std::num::NonZeroUsize,
    usize,
    non_max_usize_test
);
