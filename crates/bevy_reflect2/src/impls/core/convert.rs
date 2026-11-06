use crate::{self as bevy_reflect2, impl_reflect_primitive};

// `!` itself is not reflected: it cannot be named on stable Rust yet ([#35121]), and implementing
// traits for it through a stable workaround (ex: `<fn() -> ! as Trait>::Output`) breaks coherence
// for downstream implementations of those traits.
//
// [#35121]: https://github.com/rust-lang/rust/issues/35121
//
// `Infallible` has no values, so its vtable functions can never be called. They are still
// provided so that types containing it (ex: `Result<T, Infallible>`) can use them structurally.
impl_reflect_primitive!(
    #[primitive(Never)]
    ::core::convert::Infallible(Clone, Debug, Hash, PartialEq, PartialOrd)
);
