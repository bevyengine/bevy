use crate::archetype::ArchetypeComponentId;
use crate::archetype::ArchetypeComponentInfo;
use crate::component::ComponentId;
use crate::storage::{Column, SparseSet};

#[derive(Default)]
pub struct Resources {
    pub(crate) resources: SparseSet<ComponentId, Column>,
    pub(crate) components: SparseSet<ComponentId, ArchetypeComponentInfo>,
}

impl Resources {
    #[inline]
    pub(crate) fn columns_mut(&mut self) -> impl Iterator<Item = &mut Column> {
        self.resources.values_mut()
    }

    #[inline]
    pub fn get_archetype_component_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        self.components
            .get(component_id)
            .map(|info| info.archetype_component_id)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    #[inline]
    pub(crate) fn get(&self, component: ComponentId) -> Option<&Column> {
        self.resources.get(component)
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, component: ComponentId) -> Option<&mut Column> {
        self.resources.get_mut(component)
    }
}
