use crate::{
    prelude::{Bundle, Trigger},
    system::System,
};

use super::IntoSystem;

/// Implemented for [`System`]s that have a [`Trigger`] as the first argument.
pub trait ObserverSystem<E: 'static, B: Bundle, Out = ()>:
    System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static
{
}

impl<
        E: 'static,
        B: Bundle,
        Out,
        T: System<In = Trigger<'static, E, B>, Out = Out> + Send + 'static,
    > ObserverSystem<E, B, Out> for T
{
}

/// Implemented for systems that convert into [`ObserverSystem`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot become an `ObserverSystem`",
    label = "the trait `IntoObserverSystem` is not implemented",
    note = "for function `ObserverSystem`s, ensure the first argument is a `Trigger<T>` and any subsequent ones are `SystemParam`"
)]
pub trait IntoObserverSystem<E: 'static, B: Bundle, M, Out = ()>: Send + 'static {
    /// The type of [`System`] that this instance converts into.
    type System: ObserverSystem<E, B, Out>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;
}

impl<
        S: IntoSystem<Trigger<'static, E, B>, Out, M> + Send + 'static,
        M,
        Out,
        E: 'static,
        B: Bundle,
    > IntoObserverSystem<E, B, M, Out> for S
where
    S::System: ObserverSystem<E, B, Out>,
{
    type System = <S as IntoSystem<Trigger<'static, E, B>, Out, M>>::System;

    fn into_system(this: Self) -> Self::System {
        IntoSystem::into_system(this)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        event::Event,
        observer::Trigger,
        system::{In, IntoSystem},
        world::World,
    };

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    fn test_piped_observer_systems_no_input() {
        fn a(_: Trigger<TriggerEvent>) {}
        fn b() {}

        let mut world = World::new();
        world.observe(a.pipe(b));
    }

    #[test]
    fn test_piped_observer_systems_with_inputs() {
        fn a(_: Trigger<TriggerEvent>) -> u32 {
            3
        }
        fn b(_: In<u32>) {}

        let mut world = World::new();
        world.observe(a.pipe(b));
    }
}
