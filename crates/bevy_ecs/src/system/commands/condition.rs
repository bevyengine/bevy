//! Contains the definition of the [`CommandCondition`] trait,
//! as well as blanket implementations for boolean values and closures.
//!
//! These conditions act as predicates for [`Commands`](crate::system::Commands)
//! and [`EntityCommands`](crate::system::EntityCommands), allowing for
//! conditional world mutations in a fluent, chainable manner.

/// A predicate used to determine if a world mutation should be applied.
///
/// Types implementing this trait can be evaluated to a boolean state. This is
/// primarily used in conditional command methods like `insert_if` or `remove_if`.
///
/// # Ownership and Lifecycle
///
/// Since [`evaluate`](Self::evaluate) takes `self` by value, the condition is
/// consumed upon evaluation. This allows closures to capture and move data
/// from their environment, making it a flexible tool for one-off system logic.
///
/// # Examples
///
/// Using a simple boolean variable:
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct Poison;
/// # fn system(mut commands: Commands, entity: Entity) {
/// let is_immune = true;
/// commands.entity(entity).insert_if(Poison, !is_immune);
/// # }
/// ```
///
/// Using a closure for lazy evaluation:
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct Health(u32);
/// # fn system(mut commands: Commands, entity: Entity) {
/// commands.entity(entity).remove_if::<Health>(|| {
///     // Complex logic evaluated at call time
///     1 + 1 == 2
/// });
/// # }
/// ```
pub trait CommandCondition: Sized {
    /// Evaluates the condition, returning `true` if the associated action should proceed.
    ///
    /// This consumes the condition.
    fn evaluate(self) -> bool;
}

impl CommandCondition for bool {
    #[inline]
    fn evaluate(self) -> bool {
        self
    }
}

impl<F> CommandCondition for F
where
    F: FnOnce() -> bool,
{
    /// Executes the closure and returns its result.
    #[inline]
    fn evaluate(self) -> bool {
        self()
    }
}
