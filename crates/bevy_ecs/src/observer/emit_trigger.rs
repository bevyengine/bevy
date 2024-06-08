use crate::{
    component::ComponentId,
    entity::Entity,
    observer::Trigger,
    world::{Command, DeferredWorld, World},
};

/// A [`Command`] that emits a given trigger for a given set of targets.
pub struct EmitTrigger<T, Targets: TriggerTargets = ()> {
    /// The trigger to emit.
    pub trigger: T,

    /// The targets to emit the trigger for.
    pub targets: Targets,
}

impl<T: Trigger, Targets: TriggerTargets> Command for EmitTrigger<T, Targets> {
    fn apply(mut self, world: &mut World) {
        let trigger_id = world.init_component::<T>();
        apply_trigger(world, trigger_id, &mut self.trigger, self.targets);
    }
}

/// Emit a trigger for a dynamic component id. This is unsafe and must be verified manually.
pub struct EmitDynamicTrigger<T, Targets: TriggerTargets = ()> {
    trigger: ComponentId,
    data: T,
    targets: Targets,
}

impl<T, Targets: TriggerTargets> EmitDynamicTrigger<T, Targets> {
    /// Sets the trigger id of the resulting trigger, used for dynamic triggers
    /// # Safety
    /// Caller must ensure that the component associated with `id` is accessible as E
    pub unsafe fn new_with_id(trigger: ComponentId, data: T, targets: Targets) -> Self {
        Self {
            trigger,
            data,
            targets,
        }
    }
}

impl<T: Trigger, Targets: TriggerTargets> Command for EmitDynamicTrigger<T, Targets> {
    fn apply(mut self, world: &mut World) {
        apply_trigger(world, self.trigger, &mut self.data, self.targets);
    }
}

#[inline]
fn apply_trigger<T, Targets: TriggerTargets>(
    world: &mut World,
    trigger_id: ComponentId,
    data: &mut T,
    targets: Targets,
) {
    let mut world = DeferredWorld::from(world);
    if targets.entities().len() == 0 {
        // SAFETY: T is accessible as the type represented by self.trigger, ensured in `Self::new`
        unsafe {
            world.trigger_observers_with_data(
                trigger_id,
                Entity::PLACEHOLDER,
                targets.components(),
                data,
            );
        };
    } else {
        for target in targets.entities() {
            // SAFETY: T is accessible as the type represented by self.trigger, ensured in `Self::new`
            unsafe {
                world.trigger_observers_with_data(trigger_id, target, targets.components(), data);
            };
        }
    }
}

/// Represents a collection of targets, which can be of type [`Entity`] or [`ComponentId`].
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
