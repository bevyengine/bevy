use crate::{bundle::Bundle, prelude::Trigger, system::System};

/// Trait for types that can be used as input to [`System`]s.
///
/// Provided implementations are:
/// - `()`: No input
/// - [`In<T>`]: For values
/// - [`InRef<T>`]: For read-only references
/// - [`InMut<T>`]: For mutable references
/// - [`Trigger<E, B>`]: For [`ObserverSystem`]s
///
/// [`ObserverSystem`]: crate::system::ObserverSystem
pub trait SystemInput: Sized {
    /// The outer input type that is defined as the first argument to systems,
    /// similar to [`SystemParam`](crate::system::SystemParam)s.
    type Param<'i>: SystemInput;
    /// The inner input type that is passed to system run functions.
    type Inner<'i>;

    /// Converts `self` into a `'static` version of [`SystemInput::Param`].
    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static;

    /// Converts a [`SystemInput::Param`] into a [`SystemInput::Inner`].
    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_>;

    /// Converts a [`SystemInput::Inner`] into a [`SystemInput::Param`].
    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_>;
}

/// Shorthand way to get the [`System::In`] for a [`System`] as a [`SystemInput::Param`].
pub type SystemInParam<'a, S> = <<S as System>::In as SystemInput>::Param<'a>;
/// Shorthand way to get the [`System::In`] for a [`System`] as a [`SystemInput::Inner`].
pub type SystemIn<'a, S> = <<S as System>::In as SystemInput>::Inner<'a>;

/// [`SystemInput`] type for systems that take no input.
impl SystemInput for () {
    type Param<'i> = ();
    type Inner<'i> = ();

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
    }

    fn to_inner(_this: Self::Param<'_>) -> Self::Inner<'_> {}

    fn to_param(_this: Self::Inner<'_>) -> Self::Param<'_> {}
}

/// Wrapper type to mark a [`SystemParam`] as an input.
///
/// [`System`]s may take an optional input which they require to be passed to them when they
/// are being [`run`](System::run). For [`FunctionSystems`](FunctionSystem) the input may be marked
/// with this `In` type, but only the first param of a function may be tagged as an input. This also
/// means a system can only have one or zero input parameters.
///
/// # Examples
///
/// Here is a simple example of a system that takes a [`usize`] returning the square of it.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn main() {
///     let mut square_system = IntoSystem::into_system(square);
///
///     let mut world = World::default();
///     square_system.initialize(&mut world);
///     assert_eq!(square_system.run(12, &mut world), 144);
/// }
///
/// fn square(In(input): In<usize>) -> usize {
///     input * input
/// }
/// ```
///
/// [`SystemParam`]: crate::system::SystemParam
pub struct In<In>(pub In);

impl<T: 'static> SystemInput for In<T> {
    type Param<'i> = In<T>;
    type Inner<'i> = T;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this.0
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        In(this)
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input which takes a read-only reference.
///
/// This is similar to [`In`] but takes a reference instead of the value itself.
/// See [`InMut`] for the mutable version.
///
/// [`SystemParam`]: crate::system::SystemParam
pub struct InRef<'i, T>(pub &'i T);

impl<T: 'static> SystemInput for InRef<'_, T> {
    type Param<'i> = InRef<'i, T>;
    type Inner<'i> = &'i T;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this.0
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        InRef(this)
    }
}

/// Wrapper type to mark a [`SystemParam`] as an input which takes a mutable reference.
///
/// This is similar to [`In`] but takes a mutable reference instead of the value itself.
/// See [`InRef`] for the read-only version.
///
/// [`SystemParam`]: crate::system::SystemParam
pub struct InMut<'a, T>(pub &'a mut T);

impl<T: 'static> SystemInput for InMut<'_, T> {
    type Param<'i> = InMut<'i, T>;
    type Inner<'i> = &'i mut T;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this.0
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        InMut(this)
    }
}

impl<E: 'static, B: Bundle> SystemInput for Trigger<'_, E, B> {
    type Param<'i> = Trigger<'i, E, B>;
    type Inner<'i> = Trigger<'i, E, B>;

    fn to_static(self) -> Self::Param<'static>
    where
        Self: 'static,
    {
        self
    }

    fn to_inner(this: Self::Param<'_>) -> Self::Inner<'_> {
        this
    }

    fn to_param(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}
