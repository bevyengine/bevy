//! Traits used by label implementations

use core::{
    any::Any,
    hash::{Hash, Hasher},
};

// Re-exported for use within `define_label!`
#[doc(hidden)]
pub use alloc::boxed::Box;

/// An object safe version of [`Eq`]. This trait is automatically implemented
/// for any `'static` type that implements `Eq`.
pub trait DynEq: Any {
    /// This method tests for `self` and `other` values to be equal.
    ///
    /// Implementers should avoid returning `true` when the underlying types are
    /// not the same.
    fn dyn_eq(&self, other: &dyn DynEq) -> bool;
}

// Tests that this trait is dyn-compatible
const _: Option<Box<dyn DynEq>> = None;

impl<T> DynEq for T
where
    T: Any + Eq,
{
    fn dyn_eq(&self, other: &dyn DynEq) -> bool {
        if let Some(other) = (other as &dyn Any).downcast_ref::<T>() {
            return self == other;
        }
        false
    }
}

/// An object safe version of [`Hash`]. This trait is automatically implemented
/// for any `'static` type that implements `Hash`.
pub trait DynHash: DynEq {
    /// Feeds this value into the given [`Hasher`].
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

// Tests that this trait is dyn-compatible
const _: Option<Box<dyn DynHash>> = None;

impl<T> DynHash for T
where
    T: DynEq + Hash,
{
    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        T::hash(self, &mut state);
        self.type_id().hash(&mut state);
    }
}

/// Macro to define a new label trait.
///
/// Each label trait has an associated [`Interner<dyn YourLabelTraitHere>`][crate::intern::Interner]
/// The trait has an `intern(&self)` method which uses that interner to
/// produce [`Interned<dyn YourLabelTraitHere>`][crate::intern::Interned] values,
/// and a `dyn_clone(&self)` method which must be implemented for the system to work.
///
/// # Examples
///
/// Minimal working example:
///
/// ```
/// # use bevy_ecs::define_label;
/// // Defines `trait MyNewLabelTrait` and `static MY_NEW_LABEL_TRAIT_INTERNER`.
/// // You don’t need to use the interner for anything; just give it a unique name.
/// define_label!(
///     /// Documentation of label trait
///     MyNewLabelTrait,
/// );
///
/// /// A new label type implementing the new label trait.
/// #[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// pub struct MyLabel;
///
/// impl MyNewLabelTrait for MyLabel {
///     // Implementations of the trait must implement the `dyn_clone()` method in this way
///     // to enable cloning the trait object because `Clone` is not `dyn` compatible.
///     fn dyn_clone(&self) -> Box<dyn MyNewLabelTrait> {
///         Box::new(self.clone())
///     }
/// }
///
/// assert_eq!(MyLabel.intern(), MyLabel.intern());
/// ```
///
/// A label trait defined by this macro can also be given additional methods:
///
/// ```
/// # use bevy_ecs::define_label;
/// define_label!(
///     /// Documentation of another label trait
///     MyNewExtendedLabelTrait,
///     extra_methods: {
///         // Extra methods for the trait can be defined here
///         fn additional_method(&self) -> i32;
///     },
///     extra_methods_impl: {
///         // Implementation of the extra methods for Interned<dyn MyNewExtendedLabelTrait>,
///         // which should usually forward to the contained value.
///         fn additional_method(&self) -> i32 {
///             (**self).additional_method()
///         }
///     }
/// );
///
/// #[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// pub struct MyLabel;
///
/// impl MyNewExtendedLabelTrait for MyLabel {
///     fn dyn_clone(&self) -> Box<dyn MyNewExtendedLabelTrait> {
///         Box::new(self.clone())
///     }
///
///     fn additional_method(&self) -> i32 {
///         42
///     }
/// }
///
/// let interned_label = MyLabel.intern();
/// assert_eq!(interned_label.additional_method(), 42);
/// ```
///
/// In order to minimize boilerplate for each new label type, you may wish to define a macro to
/// generate labels. In Bevy’s own traits, this is done by derive macros (e.g.
/// `derive(ScheduleLabel)`), but it is often sufficient to write a simple, less general
/// `macro_rules!` macro:
///
/// ```
/// # use bevy_ecs::define_label;
/// define_label!(Team);
///
/// macro_rules! define_team {
///     ($name:ident) => {
///         #[derive(Clone, Debug, Eq, Hash, PartialEq)]
///         pub struct $name;
///
///         impl Team for $name {
///             fn dyn_clone(&self) -> Box<dyn Team> {
///                 Box::new(self.clone())
///             }
///         }
///     }
/// }
///
/// define_team!(Home);
/// define_team!(Away);
///
/// assert_eq!(Home.intern(), Home.intern());
/// assert_ne!(Home.intern(), Away.intern());
/// ```
///
#[macro_export]
macro_rules! define_label {
    (
        $(#[$label_attr:meta])*
        $label_trait_name:ident $(,)?
    ) => {
        $crate::define_label!(
            $(#[$label_attr])*
            $label_trait_name,
            extra_methods: {},
            extra_methods_impl: {}
        );
    };
    (
        $(#[$label_attr:meta])*
        $label_trait_name:ident,
        extra_methods: { $($trait_extra_methods:tt)* },
        extra_methods_impl: { $($interned_extra_methods_impl:tt)* }  $(,)?
    ) => {

        $(#[$label_attr])*
        pub trait $label_trait_name: ::core::marker::Send + ::core::marker::Sync + ::core::fmt::Debug + $crate::label::DynEq + $crate::label::DynHash {

            $($trait_extra_methods)*

            /// Clones this `
            #[doc = ::core::stringify!($label_trait_name)]
            ///`.
            fn dyn_clone(&self) -> $crate::label::Box<dyn $label_trait_name>;

            /// Returns an [`Interned`] value corresponding to `self`.
            fn intern(&self) -> $crate::intern::Interned<dyn $label_trait_name>
            where Self: ::core::marker::Sized {
                static INTERNER: $crate::intern::Interner<dyn $label_trait_name> =
                    $crate::intern::Interner::new();

                INTERNER.intern(self)
            }
        }

        #[diagnostic::do_not_recommend]
        impl $label_trait_name for $crate::intern::Interned<dyn $label_trait_name> {

            $($interned_extra_methods_impl)*

            fn dyn_clone(&self) -> $crate::label::Box<dyn $label_trait_name> {
                (**self).dyn_clone()
            }

            fn intern(&self) -> Self {
                *self
            }
        }

        impl ::core::cmp::PartialEq for dyn $label_trait_name {
            fn eq(&self, other: &Self) -> bool {
                self.dyn_eq(other)
            }
        }

        impl ::core::cmp::Eq for dyn $label_trait_name {}

        impl ::core::hash::Hash for dyn $label_trait_name {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                self.dyn_hash(state);
            }
        }

        impl $crate::intern::Internable for dyn $label_trait_name {
            fn leak(&self) -> &'static Self {
                $crate::label::Box::leak(self.dyn_clone())
            }

            fn ref_eq(&self, other: &Self) -> bool {
                use ::core::ptr;

                // Test that both the type id and pointer address are equivalent.
                self.type_id() == other.type_id()
                    && ptr::addr_eq(ptr::from_ref::<Self>(self), ptr::from_ref::<Self>(other))
            }

            fn ref_hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                use ::core::{hash::Hash, ptr};

                // Hash the type id...
                self.type_id().hash(state);

                // ...and the pointer address.
                // Cast to a unit `()` first to discard any pointer metadata.
                ptr::from_ref::<Self>(self).cast::<()>().hash(state);
            }
        }
    };
}
