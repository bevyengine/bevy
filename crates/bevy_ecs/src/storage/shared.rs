use bevy_platform::{collections::HashSet, sync::Arc};
use core::{
    cell::Cell,
    hash::{Hash, Hasher},
    ops::Deref,
};
use indexmap::Equivalent;

use crate::{
    component::{ComponentId, Tick},
    fragmenting_value::FragmentingValue,
};

pub struct SharedComponent {
    component_id: ComponentId,
    added: Cell<Tick>,
    value: SharedFragmentingValue,
}

impl SharedComponent {
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    pub fn added(&self) -> Tick {
        self.added.get()
    }

    pub fn check_tick(&self, change_tick: Tick) -> bool {
        let mut tick = self.added.get();
        let wrapped = tick.check_tick(change_tick);
        self.added.set(tick);
        return wrapped;
    }

    pub fn value(&self) -> &SharedFragmentingValue {
        &self.value
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

#[derive(Default)]
pub struct Shared {
    values_set: HashSet<SharedComponent>,
}

impl Shared {
    pub fn get_or_insert(
        &mut self,
        current_tick: Tick,
        component_id: ComponentId,
        value: &dyn FragmentingValue,
    ) -> &SharedComponent {
        self.values_set
            .get_or_insert_with(value, |key| SharedComponent {
                component_id,
                added: Cell::new(current_tick),
                value: SharedFragmentingValue(Arc::from(key.clone_boxed())),
            })
    }

    pub fn get(&self, value: &dyn FragmentingValue) -> Option<&SharedComponent> {
        self.values_set.get(value)
    }

    pub fn clear(&mut self) {
        self.values_set.clear()
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for component in self.values_set.iter() {
            component.check_tick(change_tick);
        }
    }
}

impl Equivalent<SharedComponent> for dyn FragmentingValue {
    fn equivalent(&self, key: &SharedComponent) -> bool {
        *self == *key.value.as_ref()
    }
}

#[derive(Clone)]
pub struct SharedFragmentingValue(Arc<dyn FragmentingValue>);

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
