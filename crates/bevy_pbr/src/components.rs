use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use bevy_ecs::entity::{Entity, EntityHashMap};
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::sync_world::MainEntity;
/// Collection of mesh entities visible for 3D lighting.
///
/// This component contains all mesh entities visible from the current light view.
/// The collection is updated automatically by [`crate::SimulationLightSystems`].
#[derive(Component, Clone, Debug, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Debug, Default, Clone)]
pub struct VisibleMeshEntities {
    #[reflect(ignore, clone)]
    pub entities: Vec<Entity>,
}

#[derive(Component, Clone, Debug, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Debug, Default, Clone)]
pub struct RenderVisibleMeshEntities {
    #[reflect(ignore, clone)]
    pub entities: Vec<(Entity, MainEntity)>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct CubemapVisibleEntities {
    #[reflect(ignore, clone)]
    data: [VisibleMeshEntities; 6],
}

impl CubemapVisibleEntities {
    pub fn get(&self, i: usize) -> &VisibleMeshEntities {
        &self.data[i]
    }

    pub fn get_mut(&mut self, i: usize) -> &mut VisibleMeshEntities {
        &mut self.data[i]
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleMeshEntities> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut VisibleMeshEntities> {
        self.data.iter_mut()
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct RenderCubemapVisibleEntities {
    #[reflect(ignore, clone)]
    pub(crate) data: [RenderVisibleMeshEntities; 6],
}

impl RenderCubemapVisibleEntities {
    pub fn get(&self, i: usize) -> &RenderVisibleMeshEntities {
        &self.data[i]
    }

    pub fn get_mut(&mut self, i: usize) -> &mut RenderVisibleMeshEntities {
        &mut self.data[i]
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &RenderVisibleMeshEntities> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut RenderVisibleMeshEntities> {
        self.data.iter_mut()
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct CascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore, clone)]
    pub entities: EntityHashMap<Vec<VisibleMeshEntities>>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct RenderCascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore, clone)]
    pub entities: EntityHashMap<Vec<RenderVisibleMeshEntities>>,
}
