use crate::{GetTypeRegistration, Reflect, TypePath, Typed};

/// A catch-all trait that is bound by the core reflection traits,
/// useful to simplify reflection-based generic type bounds.
///
/// You do _not_ need to implement this trait manually.
/// It is automatically implemented for all types that implement its supertraits.
/// And these supertraits are all automatically derived with the [`Reflect` derive macro].
///
/// This should namely be used to bound generic arguments to the necessary traits for reflection.
/// Doing this has the added benefit of reducing migration costs, as a change to the required traits
/// is automatically handled by this trait.
///
/// For now, the supertraits of this trait includes:
/// * [`Reflect`]
/// * [`GetTypeRegistration`]
/// * [`Typed`]
/// * [`TypePath`]
///
/// ## Example
///
/// ```
/// # use bevy_reflect::{Reflect, Reflectable};
/// #[derive(Reflect)]
/// struct MyStruct<T: Reflectable> {
///     value: T
/// }
/// ```
///
/// [`Reflect` derive macro]: bevy_reflect_derive::Reflect
pub trait Reflectable: Reflect + GetTypeRegistration + Typed + TypePath {}

impl<T: Reflect + GetTypeRegistration + Typed + TypePath> Reflectable for T {}
