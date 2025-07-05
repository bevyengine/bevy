pub use bevy_camera::visibility::{
    CascadesVisibleEntities, CubemapVisibleEntities, VisibleMeshEntities,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use bevy_ecs::entity::{Entity, EntityHashMap};
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::sync_world::MainEntity;

#[derive(Component, Clone, Debug, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Debug, Default, Clone)]
pub struct RenderVisibleMeshEntities {
    #[reflect(ignore, clone)]
    pub entities: Vec<(Entity, MainEntity)>,
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
pub struct RenderCascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore, clone)]
    pub entities: EntityHashMap<Vec<RenderVisibleMeshEntities>>,
}
