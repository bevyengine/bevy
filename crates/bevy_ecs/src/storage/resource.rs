use crate::component::ComponentId;
use crate::archetype::ArchetypeComponentInfo;
use crate::storage::{SparseSet, Column};

#[derive(Default)]
pub struct Resources {
    resources: SparseSet<ComponentId, Column>,
    components: SparseSet<ComponentId, ArchetypeComponentInfo>,
}

impl Resources {
    #[inline]
    pub(crate) fn columns_mut(&mut self) -> impl Iterator<Item = &mut Column>  {
        self.resources.values_mut()
    }

    #[inline]
    pub(crate) fn components_mut(&mut self) -> &mut SparseSet<ComponentId, ArchetypeComponentInfo>  {
        &mut self.components
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    #[inline]
    pub(crate) fn get(&self, component: ComponentId) -> Option<&Column>  {
        self.resources.get(component)
    }

    #[inline]
    pub(crate) fn get_mut(&mut self, component: ComponentId) -> Option<&mut Column>  {
        self.resources.get_mut(component)
    }
}
