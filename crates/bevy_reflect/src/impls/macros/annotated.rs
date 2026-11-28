/// Invokes `impl_reflect_opaque!` with an extra custom attribute when the `annotations` feature
/// is enabled, and without it otherwise.
macro_rules! annotated {
    (@$annotation:expr; impl_reflect_opaque!(::$($segment:ident)::+ ($($traits:ident),* $(,)?))) => {
        #[cfg(feature = "annotations")]
        impl_reflect_opaque!(::$($segment)::+ ($($traits,)* @$annotation));
        #[cfg(not(feature = "annotations"))]
        impl_reflect_opaque!(::$($segment)::+ ($($traits),*));
    };
}

pub(crate) use annotated;
