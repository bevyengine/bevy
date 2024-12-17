#[cfg(feature = "track_change_detection")]
use core::panic::Location;

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

    /// The source code that emitted this command.
    #[cfg(feature = "track_change_detection")]
    pub caller: &'static Location<'static>,
}

impl<E: Event, Targets: TriggerTargets> TriggerEvent<E, Targets> {
    pub(super) fn trigger(mut self, world: &mut World) {
        let event_type = world.register_component::<E>();
        trigger_event(
            world,
            event_type,
            &mut self.event,
            self.targets,
            #[cfg(feature = "track_change_detection")]
            self.caller,
        );
    }
}

impl<E: Event, Targets: TriggerTargets> TriggerEvent<&mut E, Targets> {
    pub(super) fn trigger_ref(self, world: &mut World) {
        let event_type = world.register_component::<E>();
        trigger_event(
            world,
            event_type,
            self.event,
            self.targets,
            #[cfg(feature = "track_change_detection")]
            self.caller,
        );
    }
}

impl<E: Event, Targets: TriggerTargets + Send + Sync + 'static> Command
    for TriggerEvent<E, Targets>
{
    fn apply(self, world: &mut World) {
        self.trigger(world);
    }
}

/// Emit a trigger for a dynamic component id. This is unsafe and must be verified manually.
pub struct EmitDynamicTrigger<T, Targets: TriggerTargets = ()> {
    event_type: ComponentId,
    event_data: T,
    targets: Targets,
    #[cfg(feature = "track_change_detection")]
    caller: &'static Location<'static>,
}

impl<E, Targets: TriggerTargets> EmitDynamicTrigger<E, Targets> {
    /// Sets the event type of the resulting trigger, used for dynamic triggers
    /// # Safety
    /// Caller must ensure that the component associated with `event_type` is accessible as E
    #[track_caller]
    pub unsafe fn new_with_id(event_type: ComponentId, event_data: E, targets: Targets) -> Self {
        Self {
            event_type,
            event_data,
            targets,
            #[cfg(feature = "track_change_detection")]
            caller: Location::caller(),
        }
    }
}

impl<E: Event, Targets: TriggerTargets + Send + Sync + 'static> Command
    for EmitDynamicTrigger<E, Targets>
{
    fn apply(mut self, world: &mut World) {
        trigger_event(
            world,
            self.event_type,
            &mut self.event_data,
            self.targets,
            #[cfg(feature = "track_change_detection")]
            self.caller,
        );
    }
}

#[inline]
fn trigger_event<E: Event, Targets: TriggerTargets>(
    world: &mut World,
    event_type: ComponentId,
    event_data: &mut E,
    targets: Targets,
    #[cfg(feature = "track_change_detection")] caller: &'static Location<'static>,
) {
    let mut world = DeferredWorld::from(world);
    if targets.entities().is_empty() {
        // SAFETY: T is accessible as the type represented by self.trigger, ensured in `Self::new`
        unsafe {
            world.trigger_observers_with_data::<_, E::Traversal>(
                event_type,
                Entity::PLACEHOLDER,
                targets.components(),
                event_data,
                false,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
        };
    } else {
        for target in targets.entities() {
            // SAFETY: T is accessible as the type represented by self.trigger, ensured in `Self::new`
            unsafe {
                world.trigger_observers_with_data::<_, E::Traversal>(
                    event_type,
                    *target,
                    targets.components(),
                    event_data,
                    E::AUTO_PROPAGATE,
                    #[cfg(feature = "track_change_detection")]
                    caller,
                );
            };
        }
    }
}

/// Represents a collection of targets for a specific [`Trigger`] of an [`Event`]. Targets can be of type [`Entity`] or [`ComponentId`].
///
/// When a trigger occurs for a given event and [`TriggerTargets`], any [`Observer`] that watches for that specific event-target combination
/// will run.
///
/// [`Trigger`]: crate::observer::Trigger
/// [`Observer`]: crate::observer::Observer
pub trait TriggerTargets {
    /// The components the trigger should target.
    fn components(&self) -> &[ComponentId];

    /// The entities the trigger should target.
    fn entities(&self) -> &[Entity];
}

impl TriggerTargets for () {
    fn components(&self) -> &[ComponentId] {
        &[]
    }

    fn entities(&self) -> &[Entity] {
        &[]
    }
}

impl TriggerTargets for Entity {
    fn components(&self) -> &[ComponentId] {
        &[]
    }

    fn entities(&self) -> &[Entity] {
        core::slice::from_ref(self)
    }
}

impl TriggerTargets for Vec<Entity> {
    fn components(&self) -> &[ComponentId] {
        &[]
    }

    fn entities(&self) -> &[Entity] {
        self.as_slice()
    }
}

impl<const N: usize> TriggerTargets for [Entity; N] {
    fn components(&self) -> &[ComponentId] {
        &[]
    }

    fn entities(&self) -> &[Entity] {
        self.as_slice()
    }
}

impl TriggerTargets for ComponentId {
    fn components(&self) -> &[ComponentId] {
        core::slice::from_ref(self)
    }

    fn entities(&self) -> &[Entity] {
        &[]
    }
}

impl TriggerTargets for Vec<ComponentId> {
    fn components(&self) -> &[ComponentId] {
        self.as_slice()
    }

    fn entities(&self) -> &[Entity] {
        &[]
    }
}

impl<const N: usize> TriggerTargets for [ComponentId; N] {
    fn components(&self) -> &[ComponentId] {
        self.as_slice()
    }

    fn entities(&self) -> &[Entity] {
        &[]
    }
}

impl TriggerTargets for &Vec<Entity> {
    fn components(&self) -> &[ComponentId] {
        &[]
    }

    fn entities(&self) -> &[Entity] {
        self.as_slice()
    }
}
