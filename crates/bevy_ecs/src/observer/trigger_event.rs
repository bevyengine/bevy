use crate::{
    component::ComponentId,
    entity::Entity,
    event::Event,
    world::{Command, DeferredWorld, World},
};

/// A [`Command`] that emits a given trigger for a given set of targets.
pub struct TriggerEvent<E, Targets: TriggerTargets = ()> {
    /// The event to trigger.
    pub event: E,

    /// The targets to trigger the event for.
    pub targets: Targets,
}

impl<E: Event, Targets: TriggerTargets> Command for TriggerEvent<E, Targets> {
    fn apply(mut self, world: &mut World) {
        let event_type = world.init_component::<E>();
        trigger_event(world, event_type, &mut self.event, self.targets);
    }
}

/// Emit a trigger for a dynamic component id. This is unsafe and must be verified manually.
pub struct EmitDynamicTrigger<T, Targets: TriggerTargets = ()> {
    event_type: ComponentId,
    event_data: T,
    targets: Targets,
}

impl<E, Targets: TriggerTargets> EmitDynamicTrigger<E, Targets> {
    /// Sets the event type of the resulting trigger, used for dynamic triggers
    /// # Safety
    /// Caller must ensure that the component associated with `event_type` is accessible as E
    pub unsafe fn new_with_id(event_type: ComponentId, event_data: E, targets: Targets) -> Self {
        Self {
            event_type,
            event_data,
            targets,
        }
    }
}

impl<E: Event, Targets: TriggerTargets> Command for EmitDynamicTrigger<E, Targets> {
    fn apply(mut self, world: &mut World) {
        trigger_event(world, self.event_type, &mut self.event_data, self.targets);
    }
}

#[inline]
fn trigger_event<E, Targets: TriggerTargets>(
    world: &mut World,
    event_type: ComponentId,
    event_data: &mut E,
    targets: Targets,
) {
    let mut world = DeferredWorld::from(world);
    if targets.entities().len() == 0 {
        // SAFETY: T is accessible as the type represented by self.trigger, ensured in `Self::new`
        unsafe {
            world.trigger_observers_with_data(
                event_type,
                Entity::PLACEHOLDER,
                targets.components(),
                event_data,
            );
        };
    } else {
        for target in targets.entities() {
            // SAFETY: T is accessible as the type represented by self.trigger, ensured in `Self::new`
            unsafe {
                world.trigger_observers_with_data(
                    event_type,
                    target,
                    targets.components(),
                    event_data,
                );
            };
        }
    }
}

/// Represents a collection of targets for a specific [`Trigger`] of an [`Event`]. Targets can be of type [`Entity`] or [`ComponentId`].
/// When a trigger occurs for a given event and [`TriggerTargets`], any [`Observer`] that watches for that specific event-target combination
/// will run.
///
/// [`Trigger`]: crate::observer::Trigger
/// [`Observer`]: crate::observer::Observer
pub trait TriggerTargets: Send + Sync + 'static {
    /// The components the trigger should target.
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId>;

    /// The entities the trigger should target.
    fn entities(&self) -> impl ExactSizeIterator<Item = Entity>;
}

impl TriggerTargets for () {
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        [].into_iter()
    }

    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        [].into_iter()
    }
}

impl TriggerTargets for Entity {
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        [].into_iter()
    }

    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        std::iter::once(*self)
    }
}

impl TriggerTargets for Vec<Entity> {
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        [].into_iter()
    }

    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        self.iter().copied()
    }
}

impl<const N: usize> TriggerTargets for [Entity; N] {
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        [].into_iter()
    }

    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        self.iter().copied()
    }
}

impl TriggerTargets for ComponentId {
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        std::iter::once(*self)
    }

    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        [].into_iter()
    }
}

impl TriggerTargets for Vec<ComponentId> {
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        self.iter().copied()
    }

    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        [].into_iter()
    }
}

impl<const N: usize> TriggerTargets for [ComponentId; N] {
    fn components(&self) -> impl ExactSizeIterator<Item = ComponentId> {
        self.iter().copied()
    }

    fn entities(&self) -> impl ExactSizeIterator<Item = Entity> {
        [].into_iter()
    }
}
