use ::alloc::boxed::Box;
use downcast_rs::{impl_downcast, Downcast};

/// A trait for representing type metadata.
///
/// Type data can be registered to the [`TypeRegistry`] and stored on a type's [`TypeRegistration`].
///
/// While type data is often generated using the [`#[reflect_trait]`](crate::reflect_trait) macro,
/// almost any type that implements [`Clone`] can be considered "type data".
/// This is because it has a blanket implementation over all `T` where `T: Clone + Send + Sync + 'static`.
///
/// For creating your own type data, see the [`CreateTypeData`] trait.
///
/// See the [crate-level documentation] for more information on type data and type registration.
///
/// [`TypeRegistry`]: crate::TypeRegistry
/// [`TypeRegistration`]: crate::TypeRegistration
/// [crate-level documentation]: crate
pub trait TypeData: Downcast + Send + Sync {
    fn clone_type_data(&self) -> Box<dyn TypeData>;
}
impl_downcast!(TypeData);

impl<T: 'static + Send + Sync> TypeData for T
where
    T: Clone,
{
    fn clone_type_data(&self) -> Box<dyn TypeData> {
        Box::new(self.clone())
    }
}

/// A trait for creating [`TypeData`].
///
/// Normally any type that is `Clone + Send + Sync + 'static` can be used as type data
/// and inserted into a [`TypeRegistration`] using [`TypeRegistration::insert`]
/// However, only types that implement this trait may be registered using [`TypeRegistry::register_type_data`],
/// [`TypeRegistry::register_type_data_with`], or via the `#[reflect(MyTrait)]` attribute with the [`Reflect` derive macro].
///
/// Note that in order to work with the `#[reflect(MyTrait)]` attribute,
/// implementors must be named with the `Reflect` prefix (e.g. `ReflectMyTrait`).
///
/// # Input
///
/// By default, this trait expects no input for creating the type data
/// (the `Input` type parameter defaults to `()`).
///
/// However, implementors may choose to implement this trait with other input types.
/// As long as the implementations don't conflict, multiple different input types can be specified.
///
/// # Example
///
/// ```
/// # use bevy_reflect::{CreateTypeData, Reflect};
/// trait Combine {
///   fn combine(a: f32, b: f32) -> f32;
/// }
///
/// #[derive(Clone)]
/// struct ReflectCombine {
///   multiplier: f32,
///   additional: f32,
///   combine: fn(f32, f32) -> f32,
/// }
///
/// impl ReflectCombine {
///   pub fn combine(&self, a: f32, b: f32) -> f32 {
///     let combined = (self.combine)(a, b);
///     let multiplied = self.multiplier * combined;
///     multiplied + self.additional
///   }
/// }
///
/// // A default implementation for when no input is given
/// impl<T: Combine + Reflect> CreateTypeData<T> for ReflectCombine {
///   fn create_type_data(_: ()) -> Self {
///     Self {
///       multiplier: 1.0,
///       additional: 0.0,
///       combine: T::combine,
///     }
///   }
/// }
///
/// // A custom implementation for when a multiplier is given
/// impl<T: Combine + Reflect> CreateTypeData<T, (f32, f32)> for ReflectCombine {
///   fn create_type_data(input: (f32, f32)) -> Self {
///     Self {
///       multiplier: input.0,
///       additional: input.1,
///       combine: T::combine,
///     }
///   }
/// }
///
/// #[derive(Reflect)]
/// // We can have the `Reflect` derive automatically register `ReflectCombine`:
/// #[reflect(Combine)]
/// struct WithoutMultiplier;
///
/// impl Combine for WithoutMultiplier {
///   fn combine(a: f32, b: f32) -> f32 {
///     a + b
///   }
/// }
///
/// #[derive(Reflect)]
/// // We can also given it some input:
/// #[reflect(Combine(2.0, 4.0))]
/// struct WithMultiplier;
///
/// impl Combine for WithMultiplier {
///   fn combine(a: f32, b: f32) -> f32 {
///     a + b
///   }
/// }
///
/// // Or we can simply create the data manually:
/// let without_multiplier = <ReflectCombine as CreateTypeData<WithoutMultiplier>>::create_type_data(());
/// let with_multiplier = <ReflectCombine as CreateTypeData<WithMultiplier, _>>::create_type_data((2.0, 4.0));
///
/// assert_eq!(without_multiplier.combine(1.0, 2.0), 3.0);
/// assert_eq!(with_multiplier.combine(1.0, 2.0), 10.0);
/// ```
///
/// [`TypeRegistration`]: crate::TypeRegistration
/// [`TypeRegistry::register_type_data`]: crate::TypeRegistry::register_type_data
/// [`TypeRegistry::register_type_data_with`]: crate::TypeRegistry::register_type_data_with
/// [`Reflect` derive macro]: derive@crate::Reflect
pub trait CreateTypeData<T, Input = ()>: TypeData {
    /// Create this type data using the given input.
    fn create_type_data(input: Input) -> Self;
}
