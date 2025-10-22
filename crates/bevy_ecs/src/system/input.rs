use core::ops::{Deref, DerefMut};

use variadics_please::all_tuples;

use crate::{bundle::Bundle, event::Event, prelude::On, system::System};

/// Trait for types that can be used as input to [`System`]s.
///
/// Provided implementations are:
/// - `()`: No input
/// - [`In<T>`]: For values
/// - [`InRef<T>`]: For read-only references to values
/// - [`InMut<T>`]: For mutable references to values
/// - [`On<E, B>`]: For [`ObserverSystem`]s
/// - [`StaticSystemInput<I>`]: For arbitrary [`SystemInput`]s in generic contexts
/// - Tuples of [`SystemInput`]s up to 8 elements
///
/// For advanced usecases, you can implement this trait for your own types.
///
/// # Examples
///
/// ## Tuples of [`SystemInput`]s
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// fn add((InMut(a), In(b)): (InMut<usize>, In<usize>)) {
///     *a += b;
/// }
/// # let mut world = World::new();
/// # let mut add = IntoSystem::into_system(add);
/// # add.initialize(&mut world);
/// # let mut a = 12;
/// # let b = 24;
/// # add.run((&mut a, b), &mut world);
/// # assert_eq!(a, 36);
/// ```
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

/// A [`SystemInput`] type which denotes that a [`System`] receives
/// an input value of type `T` from its caller.
///
/// [`System`]s may take an optional input which they require to be passed to them when they
/// are being [`run`](System::run). For [`FunctionSystem`]s the input may be marked
/// with this `In` type, but only the first param of a function may be tagged as an input. This also
/// means a system can only have one or zero input parameters.
///
/// See [`SystemInput`] to learn more about system inputs in general.
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
/// assert_eq!(square_system.run(12, &mut world).unwrap(), 144);
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
/// See [`SystemInput`] to learn more about system inputs in general.
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
/// See [`SystemInput`] to learn more about system inputs in general.
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
impl<E: Event, B: Bundle> SystemInput for On<'_, '_, E, B> {
    // Note: the fact that we must use a shared lifetime here is
    // a key piece of the complicated safety story documented above
    // the `&mut E::Trigger<'_>` cast in `observer_system_runner` and in
    // the `On` implementation.
    type Param<'i> = On<'i, 'i, E, B>;
    type Inner<'i> = On<'i, 'i, E, B>;

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
///
/// See [`SystemInput`] to learn more about system inputs in general.
pub struct StaticSystemInput<'a, I: SystemInput>(pub I::Inner<'a>);

impl<'a, I: SystemInput> SystemInput for StaticSystemInput<'a, I> {
    type Param<'i> = StaticSystemInput<'i, I>;
    type Inner<'i> = I::Inner<'i>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        StaticSystemInput(this)
    }
}

macro_rules! impl_system_input_tuple {
    ($(#[$meta:meta])* $($name:ident),*) => {
        $(#[$meta])*
        impl<$($name: SystemInput),*> SystemInput for ($($name,)*) {
            type Param<'i> = ($($name::Param<'i>,)*);
            type Inner<'i> = ($($name::Inner<'i>,)*);

            #[expect(
                clippy::allow_attributes,
                reason = "This is in a macro; as such, the below lints may not always apply."
            )]
            #[allow(
                non_snake_case,
                reason = "Certain variable names are provided by the caller, not by us."
            )]
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples won't have anything to wrap."
            )]
            fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
                let ($($name,)*) = this;
                ($($name::wrap($name),)*)
            }
        }
    };
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_system_input_tuple,
    0,
    8,
    I
);

#[cfg(test)]
mod tests {
    use crate::{
        system::{In, InMut, InRef, IntoSystem, System},
        world::World,
    };

    #[test]
    fn two_tuple() {
        fn by_value((In(a), In(b)): (In<usize>, In<usize>)) -> usize {
            a + b
        }
        fn by_ref((InRef(a), InRef(b)): (InRef<usize>, InRef<usize>)) -> usize {
            *a + *b
        }
        fn by_mut((InMut(a), In(b)): (InMut<usize>, In<usize>)) {
            *a += b;
        }

        let mut world = World::new();
        let mut by_value = IntoSystem::into_system(by_value);
        let mut by_ref = IntoSystem::into_system(by_ref);
        let mut by_mut = IntoSystem::into_system(by_mut);

        by_value.initialize(&mut world);
        by_ref.initialize(&mut world);
        by_mut.initialize(&mut world);

        let mut a = 12;
        let b = 24;

        assert_eq!(by_value.run((a, b), &mut world).unwrap(), 36);
        assert_eq!(by_ref.run((&a, &b), &mut world).unwrap(), 36);
        by_mut.run((&mut a, b), &mut world).unwrap();
        assert_eq!(a, 36);
    }
}
