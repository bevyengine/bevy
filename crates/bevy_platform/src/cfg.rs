//! Provides helpful configuration macros, allowing detection of platform features such as
//! [`alloc`](crate::cfg::alloc) or [`std`](crate::cfg::std) without explicit features.

/// Provides a `match`-like expression similar to [`cfg_if`] and based on the experimental
/// [`cfg_match`].
/// The name `switch` is used to avoid conflict with the `match` keyword.
/// Arms are evaluated top to bottom, and an optional wildcard arm can be provided if no match
/// can be made.
///
/// An arm can either be:
/// - a `cfg(...)` pattern (e.g., `feature = "foo"`)
/// - a wildcard `_`
/// - an alias defined using [`define_alias`]
///
/// Common aliases are provided by [`cfg`](crate::cfg).
/// Note that aliases are evaluated from the context of the defining crate, not the consumer.
///
/// # Examples
///
/// ```
/// # use bevy_platform::cfg;
/// # fn log(_: &str) {}
/// # fn foo(_: &str) {}
/// #
/// cfg::switch! {
///     #[cfg(feature = "foo")] => {
///         foo("We have the `foo` feature!")
///     }
///     cfg::std => {
///         extern crate std;
///         std::println!("No `foo`, but we have `std`!");
///     }
///     _ => {
///         log("Don't have `std` or `foo`");
///     }
/// }
/// ```
///
/// [`cfg_if`]: https://crates.io/crates/cfg-if
/// [`cfg_match`]: https://github.com/rust-lang/rust/issues/115585
#[doc(inline)]
pub use crate::switch;

/// Defines an alias for a particular configuration.
/// This has two advantages over directly using `#[cfg(...)]`:
///
/// 1. Complex configurations can be abbreviated to more meaningful shorthand.
/// 2. Features are evaluated in the context of the _defining_ crate, not the consuming.
///
/// The second advantage is a particularly powerful tool, as it allows consuming crates to use
/// functionality in a defining crate regardless of what crate in the dependency graph enabled the
/// relevant feature.
///
/// For example, consider a crate `foo` that depends on another crate `bar`.
/// `bar` has a feature "`faster_algorithms`".
/// If `bar` defines a "`faster_algorithms`" alias:
///
/// ```ignore
/// define_alias! {
///     #[cfg(feature = "faster_algorithms")] => { faster_algorithms }
/// }
/// ```
///
/// Now, `foo` can gate its usage of those faster algorithms on the alias, avoiding the need to
/// expose its own "`faster_algorithms`" feature.
/// This also avoids the unfortunate situation where one crate activates "`faster_algorithms`" on
/// `bar` without activating that same feature on `foo`.
///
/// Once an alias is defined, there are 4 ways you can use it:
///
/// 1. Evaluate with no contents to return a `bool` indicating if the alias is active.
///    ```
///    # use bevy_platform::cfg;
///    if cfg::std!() {
///        // Have `std`!
///    } else {
///        // No `std`...
///    }
///    ```
/// 2. Pass a single code block which will only be compiled if the alias is active.
///    ```
///    # use bevy_platform::cfg;
///    cfg::std! {
///        // Have `std`!
///    # ()
///    }
///    ```
/// 3. Pass a single `if { ... } else { ... }` expression to conditionally compile either the first
///    or the second code block.
///    ```
///    # use bevy_platform::cfg;
///    cfg::std! {
///        if {
///            // Have `std`!
///        } else {
///            // No `std`...
///        }
///    }
///    ```
/// 4. Use in a [`switch`] arm for more complex conditional compilation.
///    ```
///    # use bevy_platform::cfg;
///    cfg::switch! {
///        cfg::std => {
///            // Have `std`!
///        }
///        cfg::alloc => {
///            // No `std`, but do have `alloc`!
///        }
///        _ => {
///            // No `std` or `alloc`...
///        }
///    }
///    ```
#[doc(inline)]
pub use crate::define_alias;

/// Macro which represents an enabled compilation condition.
#[doc(inline)]
pub use crate::enabled;

/// Macro which represents a disabled compilation condition.
#[doc(inline)]
pub use crate::disabled;

#[doc(hidden)]
#[macro_export]
macro_rules! switch {
    ({ $($tt:tt)* }) => {{
        $crate::switch! { $($tt)* }
    }};
    (_ => { $($output:tt)* }) => {
        $($output)*
    };
    (
        $cond:path => $output:tt
        $($( $rest:tt )+)?
    ) => {
        $cond! {
            if {
                $crate::switch! { _ => $output }
            } else {
                $(
                    $crate::switch! { $($rest)+ }
                )?
            }
        }
    };
    (
        #[cfg($cfg:meta)] => $output:tt
        $($( $rest:tt )+)?
    ) => {
        #[cfg($cfg)]
        $crate::switch! { _ => $output }
        $(
            #[cfg(not($cfg))]
            $crate::switch! { $($rest)+ }
        )?
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! disabled {
    () => { false };
    (if { $($p:tt)* } else { $($n:tt)* }) => { $($n)* };
    ($($p:tt)*) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! enabled {
    () => { true };
    (if { $($p:tt)* } else { $($n:tt)* }) => { $($p)* };
    ($($p:tt)*) => { $($p)* };
}

#[doc(hidden)]
#[macro_export]
macro_rules! define_alias {
    (
        #[cfg($meta:meta)] => $p:ident
        $(, $( $rest:tt )+)?
    ) => {
        $crate::define_alias! {
            #[cfg($meta)] => { $p }
            $(
                $($rest)+
            )?
        }
    };
    (
        #[cfg($meta:meta)] => $p:ident,
        $($( $rest:tt )+)?
    ) => {
        $crate::define_alias! {
            #[cfg($meta)] => { $p }
            $(
                $($rest)+
            )?
        }
    };
    (
        #[cfg($meta:meta)] => {
            $(#[$p_meta:meta])*
            $p:ident
        }
        $($( $rest:tt )+)?
    ) => {
        $crate::switch! {
            #[cfg($meta)] => {
                $(#[$p_meta])*
                #[doc(inline)]
                ///
                #[doc = concat!("This macro passes the provided code because `#[cfg(", stringify!($meta), ")]` is currently active.")]
                pub use $crate::enabled as $p;
            }
            _ => {
                $(#[$p_meta])*
                #[doc(inline)]
                ///
                #[doc = concat!("This macro suppresses the provided code because `#[cfg(", stringify!($meta), ")]` is _not_ currently active.")]
                pub use $crate::disabled as $p;
            }
        }

        $(
            $crate::define_alias! {
                $($rest)+
            }
        )?
    }
}

define_alias! {
    #[cfg(feature = "alloc")] => {
        /// Indicates the `alloc` crate is available and can be used.
        alloc
    }
    #[cfg(feature = "std")] => {
        /// Indicates the `std` crate is available and can be used.
        std
    }
    #[cfg(panic = "unwind")] => {
        /// Indicates that a [`panic`] will be unwound, and can be potentially caught.
        panic_unwind
    }
    #[cfg(panic = "abort")] => {
        /// Indicates that a [`panic`] will lead to an abort, and cannot be caught.
        panic_abort
    }
    #[cfg(all(target_arch = "wasm32", feature = "web"))] => {
        /// Indicates that this target has access to browser APIs.
        web
    }
    #[cfg(all(feature = "alloc", target_has_atomic = "ptr"))] => {
        /// Indicates that this target has access to a native implementation of `Arc`.
        arc
    }
    #[cfg(feature = "critical-section")] => {
        /// Indicates `critical-section` is available.
        critical_section
    }
}
