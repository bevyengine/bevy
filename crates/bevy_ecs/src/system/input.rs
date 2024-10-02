use core::ops::{Deref, DerefMut};

use crate::{bundle::Bundle, prelude::Trigger, system::System};

/// Trait for types that can be used as input to [`System`]s.
///
/// Provided implementations are:
/// - `()`: No input
/// - [`In<T>`]: For values
/// - [`InRef<T>`]: For read-only references to values
/// - [`InMut<T>`]: For mutable references to values
/// - [`Trigger<E, B>`]: For [`ObserverSystem`]s
/// - [`StaticSystemInput<I>`]: For arbitrary [`SystemInput`]s in generic contexts
///
/// [`ObserverSystem`]: crate::system::ObserverSystem
pub trait SystemInput: Sized {
    /// The wrapper input type that is defined as the first argument to [`FunctionSystem`]s.
    ///
    /// [`FunctionSystem`]: crate::system::FunctionSystem
    type Param<'i>: SystemInput;
    /// The inner input type that is passed to functions that run systems,
    /// such as [`System::run`].
    ///
    /// [`System::run`]: crate::system::System::run
    type Inner<'i>;

    /// Converts a [`SystemInput::Inner`] into a [`SystemInput::Param`].
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_>;
}

/// Shorthand way to get the [`System::In`] for a [`System`] as a [`SystemInput::Inner`].
pub type SystemIn<'a, S> = <<S as System>::In as SystemInput>::Inner<'a>;

/// [`SystemInput`] type for systems that take no input.
impl SystemInput for () {
    type Param<'i> = ();
    type Inner<'i> = ();

    fn wrap(_this: Self::Inner<'_>) -> Self::Param<'_> {}
}

/// A [`SystemInput`] type which denotes that a [`System`] receives
/// an input value of type `T` from its caller.
///
/// [`System`]s may take an optional input which they require to be passed to them when they
/// are being [`run`](System::run). For [`FunctionSystem`]s the input may be marked
/// with this `In` type, but only the first param of a function may be tagged as an input. This also
/// means a system can only have one or zero input parameters.
///
/// # Examples
///
/// Here is a simple example of a system that takes a [`usize`] and returns the square of it.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// fn square(In(input): In<usize>) -> usize {
///     input * input
/// }
///
/// let mut world = World::new();
/// let mut square_system = IntoSystem::into_system(square);
/// square_system.initialize(&mut world);
///
/// assert_eq!(square_system.run(12, &mut world), 144);
/// ```
///
/// [`SystemParam`]: crate::system::SystemParam
/// [`FunctionSystem`]: crate::system::FunctionSystem
#[derive(Debug)]
pub struct In<T>(pub T);

impl<T: 'static> SystemInput for In<T> {
    type Param<'i> = In<T>;
    type Inner<'i> = T;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        In(this)
    }
}

impl<T> Deref for In<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for In<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A [`SystemInput`] type which denotes that a [`System`] receives
/// a read-only reference to a value of type `T` from its caller.
///
/// This is similar to [`In`] but takes a reference to a value instead of the value itself.
/// See [`InMut`] for the mutable version.
///
/// # Examples
///
/// Here is a simple example of a system that logs the passed in message.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use std::fmt::Write as _;
/// #
/// #[derive(Resource, Default)]
/// struct Log(String);
///
/// fn log(InRef(msg): InRef<str>, mut log: ResMut<Log>) {
///     writeln!(log.0, "{}", msg).unwrap();
/// }
///
/// let mut world = World::new();
/// world.init_resource::<Log>();
/// let mut log_system = IntoSystem::into_system(log);
/// log_system.initialize(&mut world);
///
/// log_system.run("Hello, world!", &mut world);
/// # assert_eq!(world.get_resource::<Log>().unwrap().0, "Hello, world!\n");
/// ```
///
/// [`SystemParam`]: crate::system::SystemParam
#[derive(Debug)]
pub struct InRef<'i, T: ?Sized>(pub &'i T);

impl<T: ?Sized + 'static> SystemInput for InRef<'_, T> {
    type Param<'i> = InRef<'i, T>;
    type Inner<'i> = &'i T;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        InRef(this)
    }
}

impl<'i, T: ?Sized> Deref for InRef<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A [`SystemInput`] type which denotes that a [`System`] receives
/// a mutable reference to a value of type `T` from its caller.
///
/// This is similar to [`In`] but takes a mutable reference to a value instead of the value itself.
/// See [`InRef`] for the read-only version.
///
/// # Examples
///
/// Here is a simple example of a system that takes a `&mut usize` and squares it.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// fn square(InMut(input): InMut<usize>) {
///     *input *= *input;
/// }
///
/// let mut world = World::new();
/// let mut square_system = IntoSystem::into_system(square);
/// square_system.initialize(&mut world);
///     
/// let mut value = 12;
/// square_system.run(&mut value, &mut world);
/// assert_eq!(value, 144);
/// ```
///
/// [`SystemParam`]: crate::system::SystemParam
#[derive(Debug)]
pub struct InMut<'a, T: ?Sized>(pub &'a mut T);

impl<T: ?Sized + 'static> SystemInput for InMut<'_, T> {
    type Param<'i> = InMut<'i, T>;
    type Inner<'i> = &'i mut T;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        InMut(this)
    }
}

impl<'i, T: ?Sized> Deref for InMut<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'i, T: ?Sized> DerefMut for InMut<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

/// Used for [`ObserverSystem`]s.
///
/// [`ObserverSystem`]: crate::system::ObserverSystem
impl<E: 'static, B: Bundle> SystemInput for Trigger<'_, E, B> {
    type Param<'i> = Trigger<'i, E, B>;
    type Inner<'i> = Trigger<'i, E, B>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

/// A helper for using [`SystemInput`]s in generic contexts.
///
/// This type is a [`SystemInput`] adapter which always has
/// `Self::Param == Self` (ignoring lifetimes for brevity),
/// no matter the argument [`SystemInput`] (`I`).
///
/// This makes it useful for having arbitrary [`SystemInput`]s in
/// function systems.
pub struct StaticSystemInput<'a, I: SystemInput>(pub I::Inner<'a>);

impl<'a, I: SystemInput> SystemInput for StaticSystemInput<'a, I> {
    type Param<'i> = StaticSystemInput<'i, I>;
    type Inner<'i> = I::Inner<'i>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        StaticSystemInput(this)
    }
}
