//! A workaround for the `!` type in stable Rust.
//!
//! This approach is taken from the [`never_say_never`] crate,
//! reimplemented here to avoid adding a new dependency.
//!
//! This module exists due to a change in [never type fallback inference] in the Rust 2024 edition.
//! This caused failures in `bevy_ecs`'s traits which are implemented for functions
//! (like [`System`](crate::system::System)) when working with panicking closures.
//!
//! Note that using this hack is not recommended in general;
//! by doing so you are knowingly opting out of rustc's stability guarantees.
//! Code that compiles due to this hack may break in future versions of Rust.
//!
//! Please read [issue #18778](https://github.com/bevyengine/bevy/issues/18778) for an explanation of why
//! Bevy has chosen to use this workaround.
//!
//! [`never_say_never`]: https://crates.io/crates/never_say_never
//! [never type fallback inference]: https://doc.rust-lang.org/edition-guide/rust-2024/never-type-fallback.html

mod fn_ret {
    /// A helper trait for naming the ! type.
    #[doc(hidden)]
    pub trait FnRet {
        /// The return type of the function.
        type Output;
    }

    /// This blanket implementation allows us to name the never type,
    /// by using the associated type of this trait for `fn() -> !`.
    impl<R> FnRet for fn() -> R {
        type Output = R;
    }
}

/// A hacky type alias for the `!` (never) type.
///
/// This knowingly opts out of rustc's stability guarantees.
/// Read the module documentation carefully before using this!
pub type Never = <fn() -> ! as fn_ret::FnRet>::Output;
