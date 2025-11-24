use crate::{
    event::Event,
    prelude::{Bundle, On},
    system::System,
};

use super::IntoSystem;

/// Implemented for [`System`]s that have [`On`] as the first argument.
pub trait ObserverSystem<E: Event, B: Bundle, Out = ()>:
    System<In = On<'static, 'static, E, B>, Out = Out> + Send + 'static
{
}

impl<E: Event, B: Bundle, Out, T> ObserverSystem<E, B, Out> for T where
    T: System<In = On<'static, 'static, E, B>, Out = Out> + Send + 'static
{
}

/// Implemented for systems that convert into [`ObserverSystem`].
///
/// # Usage notes
///
/// This trait should only be used as a bound for trait implementations or as an
/// argument to a function. If an observer system needs to be returned from a
/// function or stored somewhere, use [`ObserverSystem`] instead of this trait.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot become an `ObserverSystem`",
    label = "the trait `IntoObserverSystem` is not implemented",
    note = "for function `ObserverSystem`s, ensure the first argument is `On<T>` and any subsequent ones are `SystemParam`"
)]
pub trait IntoObserverSystem<E: Event, B: Bundle, M, Out = ()>: Send + 'static {
    /// The type of [`System`] that this instance converts into.
    type System: ObserverSystem<E, B, Out>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

impl<E: Event, B, M, Out, S> IntoObserverSystem<E, B, M, Out> for S
where
    S: IntoSystem<On<'static, 'static, E, B>, Out, M> + Send + 'static,
    S::System: ObserverSystem<E, B, Out>,
    E: 'static,
    B: Bundle,
{
    type System = S::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event::Event,
        observer::On,
        system::{In, IntoSystem},
        world::World,
    };

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    fn test_piped_observer_systems_no_input() {
        fn a(_: On<TriggerEvent>) {}
        fn b() {}

        let mut world = World::new();
        world.add_observer(a.pipe(b));
    }

    #[test]
    fn test_piped_observer_systems_with_inputs() {
        fn a(_: On<TriggerEvent>) -> u32 {
            3
        }
        fn b(_: In<u32>) {}

        let mut world = World::new();
        world.add_observer(a.pipe(b));
    }
}
