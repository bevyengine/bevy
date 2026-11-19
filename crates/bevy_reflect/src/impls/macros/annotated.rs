/// Invokes a reflection macro with an extra custom attribute when the `annotations` feature is
/// enabled, and without it otherwise.
macro_rules! annotated {
    (@$annotation:expr; impl_reflect_opaque!(::$($segment:ident)::+ ($($traits:ident),* $(,)?))) => {
        #[cfg(feature = "annotations")]
        impl_reflect_opaque!(::$($segment)::+ ($($traits,)* @$annotation));
        #[cfg(not(feature = "annotations"))]
        impl_reflect_opaque!(::$($segment)::+ ($($traits),*));
    };
    (@$annotation:expr; $macro:ident!($($body:tt)*)) => {
        #[cfg(feature = "annotations")]
        $macro!(#[reflect(@$annotation)] $($body)*);
        #[cfg(not(feature = "annotations"))]
        $macro!($($body)*);
    };
}

pub(crate) use annotated;
