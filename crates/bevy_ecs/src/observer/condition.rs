//! Run conditions for observers.
//!
//! This module provides the types needed to add run conditions to observers,
//! allowing them to conditionally execute based on world state.

use alloc::{boxed::Box, vec::Vec};
use core::marker::PhantomData;

use crate::{
    bundle::Bundle,
    event::Event,
    schedule::{BoxedCondition, SystemCondition},
    system::{IntoObserverSystem, IntoSystem},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

/// Stores a boxed condition system for an observer.
pub(crate) struct ObserverCondition {
    condition: BoxedCondition,
}

impl ObserverCondition {
    pub(crate) fn new<M>(condition: impl SystemCondition<M>) -> Self {
        Self {
            condition: Box::new(IntoSystem::into_system(condition)),
        }
    }

    pub(crate) fn from_boxed(condition: BoxedCondition) -> Self {
        Self { condition }
    }

    pub(crate) fn initialize(&mut self, world: &mut World) {
        self.condition.initialize(world);
    }

    /// # Safety
    /// - The condition must be initialized.
    /// - The world cell must have valid access for the condition's read-only parameters.
    pub(crate) unsafe fn check(&mut self, world: UnsafeWorldCell) -> bool {
        // SAFETY: Caller ensures world is valid and condition is initialized.
        // Conditions are read-only systems, so they won't cause aliasing issues.
        unsafe { self.condition.run_unsafe((), world) }.unwrap_or(false)
    }
}

#[doc(hidden)]
pub struct ObserverWithConditionMarker;

/// An observer system with run conditions that preserves event type information.
///
/// This type is returned by [`ObserverSystemExt::run_if`](super::ObserverSystemExt::run_if)
/// and allows `entity.observe(system.run_if(cond))` to work with compile-time
/// verification that the event implements [`EntityEvent`](crate::event::EntityEvent).
pub struct ObserverWithCondition<E: Event, B: Bundle, M, S: IntoObserverSystem<E, B, M>> {
    pub(crate) system: S,
    pub(crate) conditions: Vec<BoxedCondition>,
    pub(crate) _marker: PhantomData<fn() -> (E, B, M)>,
}

impl<E: Event, B: Bundle, M, S: IntoObserverSystem<E, B, M>> ObserverWithCondition<E, B, M, S> {
    /// Adds another run condition to this observer.
    ///
    /// All conditions must return `true` for the observer to run (AND semantics).
    ///
    /// **Note:** Chained `.run_if()` calls do **not** short-circuit — all conditions
    /// run every time to maintain correct change detection ticks. If you need
    /// short-circuit behavior, use `.run_if(a.and(b))`, but be aware this may cause
    /// stale `Changed<T>` detection if the second condition is frequently skipped.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Event)]
    /// # struct MyEvent;
    /// # #[derive(Resource)]
    /// # struct CondA(bool);
    /// # #[derive(Resource)]
    /// # struct CondB(bool);
    /// # fn on_event(_: On<MyEvent>) {}
    /// # let mut world = World::new();
    /// # world.insert_resource(CondA(true));
    /// # world.insert_resource(CondB(true));
    /// world.add_observer(
    ///     on_event
    ///         .run_if(|a: Res<CondA>| a.0)
    ///         .run_if(|b: Res<CondB>| b.0)
    /// );
    /// ```
    pub fn run_if<C, CM>(mut self, condition: C) -> Self
    where
        C: SystemCondition<CM>,
    {
        self.conditions
            .push(Box::new(IntoSystem::into_system(condition)));
        self
    }

    pub(crate) fn take_conditions(self) -> (S, Vec<ObserverCondition>) {
        let conditions = self
            .conditions
            .into_iter()
            .map(ObserverCondition::from_boxed)
            .collect();
        (self.system, conditions)
    }
}
