use bevy_ptr::{OwningPtr, Ptr};

use crate::archetype::ArchetypeComponentId;
use crate::component::{ComponentId, ComponentTicks, Components, Tick};
use crate::non_unique_resource::NonUniqueResourceRef;
use crate::storage::{Column, SparseSet, TableRow};

pub(crate) struct NonUniqueResourceEntry<T> {
    pub(crate) value: Option<T>,
}

struct NonUniqueResourceData {
    /// Of `NonUniqueResourceEntry<T>`.
    table: Column,
}

#[derive(Default)]
pub(crate) struct NonUniqueResources {
    resources: SparseSet<ComponentId, NonUniqueResourceData>,
}

impl NonUniqueResources {
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for resource in self.resources.values_mut() {
            resource.table.check_change_ticks(change_tick);
        }
    }

    pub(crate) fn new_with<T: Sync + Send + 'static>(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        archetype_component_id: ArchetypeComponentId,
    ) -> NonUniqueResourceRef<T> {
        let component = components.get_info(component_id).unwrap();
        let resource_data =
            self.resources
                .get_or_insert_with(component_id, || NonUniqueResourceData {
                    table: Column::with_capacity(component, 1),
                });
        let index = TableRow::new(resource_data.table.len());
        OwningPtr::make(NonUniqueResourceEntry::<T> { value: None }, |ptr| {
            // SAFETY: assuming `new_with` is called with matching `T` and `component_id`.
            unsafe {
                resource_data
                    .table
                    .push(ptr, ComponentTicks::new(Tick::new(0)));
            };
        });
        NonUniqueResourceRef::new(component_id, index, archetype_component_id)
    }

    pub(crate) unsafe fn get(&self, component_id: ComponentId, index: TableRow) -> Ptr<'_> {
        self.resources
            .get(component_id)
            .unwrap()
            .table
            .get(index)
            .unwrap()
            .0
    }
}
