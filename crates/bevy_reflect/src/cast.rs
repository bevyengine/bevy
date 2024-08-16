//! Traits for casting types to [`dyn PartialReflect`] and [`dyn Reflect`] trait objects.
//!
//! These traits are primarily used by [`ReflectBox`] to cast the inner type to
//! the corresponding reflection trait object.
//!
//! # Example
//!
//! ```
//! use bevy_reflect::{PartialReflect, Reflect, TypePath};
//! use bevy_reflect::cast::{ToPartialReflect, ToReflect};
//! use bevy_reflect::boxed::ReflectBox;
//!
//! // A custom trait used to represent equippable items
//! trait Equippable: Reflect {}
//!
//! impl TypePath for dyn Equippable {
//!     fn type_path() -> &'static str {
//!         "dyn my_crate::equipment::Equippable"
//!     }
//!
//!     fn short_type_path() -> &'static str {
//!         "dyn Equippable"
//!     }
//! }
//!
//! impl ToPartialReflect for dyn Equippable {
//!     fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
//!         self.as_partial_reflect()
//!     }
//!
//!     fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
//!         self.as_partial_reflect_mut()
//!     }
//!
//!     fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
//!         self.into_partial_reflect()
//!     }
//! }
//!
//! impl ToReflect for dyn Equippable {
//!     fn to_reflect_ref(&self) -> &dyn Reflect {
//!         self.as_reflect()
//!     }
//!
//!     fn to_reflect_mut(&mut self) -> &mut dyn Reflect {
//!         self.as_reflect_mut()
//!     }
//!
//!     fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect> {
//!         self.into_reflect()
//!     }
//! }
//!
//! #[derive(Reflect)]
//! #[reflect(from_reflect = false)]
//! struct Player {
//!     // Now `dyn Equippable` can be used with `ReflectBox`:
//!     #[reflect(remote = ReflectBox<dyn Equippable>)]
//!     weapon: Box<dyn Equippable>,
//! }
//! ```
//!
//! [`dyn PartialReflect`]: PartialReflect
//! [`dyn Reflect`]: Reflect

use crate::{PartialReflect, Reflect};

/// A trait used to access `Self` as a [`dyn PartialReflect`].
///
/// This is used by [`ReflectBox<T>`] in order to remotely reflect the inner type, `T`.
/// In most cases, [`PartialReflect`] should be used instead of this trait.
///
/// This trait can be implemented on custom trait objects to allow them to be remotely reflected
/// by [`ReflectBox`].
///
/// See the [module-level documentation] for details.
///
/// [`dyn PartialReflect`]: PartialReflect
/// [`ReflectBox<T>`]: crate::boxed::ReflectBox
/// [module-level documentation]: crate::cast
pub trait ToPartialReflect: Send + Sync + 'static {
    /// Get a reference to `Self` as a [`&dyn PartialReflect`](PartialReflect).
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect;
    /// Get a mutable reference to `Self` as a [`&mut dyn PartialReflect`](PartialReflect).
    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect;
    /// Take `Self` as a [`Box<dyn PartialReflect>`](PartialReflect).
    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect>;
}

impl<T: PartialReflect> ToPartialReflect for T {
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
        self
    }

    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }
}

impl ToPartialReflect for dyn PartialReflect {
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
        self
    }

    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }
}

/// A trait used to access `Self` as a [`dyn Reflect`].
///
/// This is used by [`ReflectBox<T>`] in order to remotely reflect the inner type, `T`.
/// In most cases, [`Reflect`] should be used instead of this trait.
///
/// This trait can be implemented on custom trait objects to allow them to be remotely reflected
/// by [`ReflectBox`].
///
/// See the [module-level documentation] for details.
///
/// [`dyn Reflect`]: Reflect
/// [`ReflectBox<T>`]: crate::boxed::ReflectBox
/// [module-level documentation]: crate::cast
pub trait ToReflect: ToPartialReflect {
    /// Get a reference to `Self` as a [`&dyn Reflect`](Reflect).
    fn to_reflect_ref(&self) -> &dyn Reflect;
    /// Get a mutable reference to `Self` as a [`&mut dyn Reflect`](Reflect).
    fn to_reflect_mut(&mut self) -> &mut dyn Reflect;
    /// Take `Self` as a [`Box<dyn Reflect>`](Reflect).
    fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect>;
}

impl<T: Reflect> ToReflect for T {
    fn to_reflect_ref(&self) -> &dyn Reflect {
        self
    }

    fn to_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }
}

impl ToPartialReflect for dyn Reflect {
    fn to_partial_reflect_ref(&self) -> &dyn PartialReflect {
        self.as_partial_reflect()
    }

    fn to_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self.as_partial_reflect_mut()
    }

    fn to_partial_reflect_box(self: Box<Self>) -> Box<dyn PartialReflect> {
        self.into_partial_reflect()
    }
}

impl ToReflect for dyn Reflect {
    fn to_reflect_ref(&self) -> &dyn Reflect {
        self
    }

    fn to_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn to_reflect_box(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }
}
