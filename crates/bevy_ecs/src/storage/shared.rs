use bevy_platform::{collections::HashSet, sync::Arc};
use core::{
    any::Any,
    cell::Cell,
    hash::{Hash, Hasher},
    ops::Deref,
};
use indexmap::Equivalent;

use crate::{
    change_detection::MaybeLocation,
    component::{ComponentId, Tick},
    fragmenting_value::FragmentingValue,
};

/// Stores data associated with a shared component.
pub struct SharedComponent {
    component_id: ComponentId,
    added: Cell<Tick>,
    location: MaybeLocation,
    value: SharedFragmentingValue,
}

impl SharedComponent {
    /// Returns [`ComponentId`] of this component.
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    /// Returns the [`Tick`] this component and value combination was first added to the world.
    pub fn added(&self) -> Tick {
        self.added.get()
    }

    /// Returns [`SharedFragmentingValue`] of this component.
    pub fn value(&self) -> &SharedFragmentingValue {
        &self.value
    }

    /// Returns [`MaybeLocation`] of this component.
    pub fn location(&self) -> &MaybeLocation {
        &self.location
    }

    /// Same as [`added`](Self::added), but returns `&Tick` instead.
    pub(crate) fn added_ref(&self) -> &Tick {
        // SAFETY:
        // The only way to obtain a `&SharedComponent` is through `Shared` storage.
        // Mutating `added` only happens in `Shared::check_change_ticks`, which requires `&mut Shared`.
        // Therefore, since &SharedComponent's lifetime is bound to &Shared's lifetime, the resulting &Tick's lifetime
        // is bound to &Shared's lifetime and there's no mutable aliasing.
        unsafe { self.added.as_ptr().as_ref().unwrap() }
    }
}

impl PartialEq for SharedComponent {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for SharedComponent {}

impl Hash for SharedComponent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

/// Stores [`SharedComponent`]s.
///
/// There is one [`SharedComponent`] per each [`FragmentingValue`] of a [`Shared`] component.
/// This acts as a central storage for these components and provides a way to do faster value comparison by
/// giving access to [`SharedFragmentingValue`], which are faster to compare than `dyn FragmentingValue`s
///
/// [`Shared`]: crate::component::StorageType::Shared
#[derive(Default)]
pub struct Shared {
    values_set: HashSet<SharedComponent>,
}

impl Shared {
    /// Get [`SharedComponent`] for the provided [`FragmentingValue`] or insert one if it doesn't exist yet.
    pub fn get_or_insert(
        &mut self,
        current_tick: Tick,
        component_id: ComponentId,
        value: &dyn FragmentingValue,
        caller: MaybeLocation,
    ) -> &SharedComponent {
        self.values_set
            .get_or_insert_with(value, |key| SharedComponent {
                component_id,
                added: Cell::new(current_tick),
                value: SharedFragmentingValue(Arc::from(key.clone_boxed())),
                location: caller,
            })
    }

    /// Get [`SharedComponent`] for the provided [`FragmentingValue`].
    ///
    /// Returns `None` if it isn't stored.
    pub fn get(&self, value: &dyn FragmentingValue) -> Option<&SharedComponent> {
        self.values_set.get(value)
    }

    /// Get [`SharedComponent`] for the provided [`SharedFragmentingValue`].
    /// Should be preferred to using [`get`](Self::get) wherever possible.
    ///
    /// Returns `None` if it isn't stored.
    pub fn get_shared(&self, value: &SharedFragmentingValue) -> Option<&SharedComponent> {
        self.values_set.get(value)
    }

    /// Clear all the stored values and reset the storage.
    ///
    /// Does not remove existing [`SharedFragmentingValue`] if they were cloned once before.
    pub fn clear(&mut self) {
        self.values_set.clear();
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for component in self.values_set.iter() {
            let mut tick = component.added.get();
            tick.check_tick(change_tick);
            component.added.set(tick);
        }
    }
}

impl Equivalent<SharedComponent> for dyn FragmentingValue {
    fn equivalent(&self, key: &SharedComponent) -> bool {
        *self == *key.value.as_ref()
    }
}

impl Equivalent<SharedComponent> for SharedFragmentingValue {
    fn equivalent(&self, key: &SharedComponent) -> bool {
        *self == key.value
    }
}

/// A version of [`FragmentingValue`] that stores the value only once and can be cloned.
///
/// Essentially, this is just a wrapper around `Arc<dyn FragmentingValue>` with some additional type safety.
#[derive(Clone)]
pub struct SharedFragmentingValue(Arc<dyn FragmentingValue>);

impl SharedFragmentingValue {
    /// Try to downcast the stored [`FragmentingValue`] to `C`.
    pub fn try_deref<C: 'static>(&self) -> Option<&C> {
        (self.0.as_ref() as &dyn Any).downcast_ref()
    }
}

impl AsRef<dyn FragmentingValue> for SharedFragmentingValue {
    fn as_ref(&self) -> &dyn FragmentingValue {
        self.0.as_ref()
    }
}

impl Deref for SharedFragmentingValue {
    type Target = dyn FragmentingValue;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq for SharedFragmentingValue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SharedFragmentingValue {}

impl Hash for SharedFragmentingValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use crate::{component::Component, world::World};

    use super::*;

    #[derive(Component, Clone, Hash, PartialEq, Eq, Debug)]
    #[component(
        key=Self,
        immutable,
        storage="Shared"
    )]
    struct SharedComponent(Vec<u32>);

    #[test]
    fn take_shared_value() {
        let mut world = World::new();
        let comp = SharedComponent(vec![1, 2, 3]);
        let e = world.spawn(comp.clone()).id();
        let taken_comp = world.entity_mut(e).take::<SharedComponent>();
        assert_eq!(taken_comp, Some(comp));
        assert!(!world.entity(e).contains::<SharedComponent>());
    }
}
