use bevy_ecs::{component::Component, entity::EntityHashMap, reflect::ReflectComponent};
use bevy_math::primitives::Frustum;
use bevy_reflect::prelude::*;

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CubemapFrusta {
    #[reflect(ignore)]
    pub frusta: [Frustum; 6],
}

impl CubemapFrusta {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Frustum> {
        self.frusta.iter()
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Frustum> {
        self.frusta.iter_mut()
    }
}

#[derive(Component, Debug, Default, Reflect, Clone)]
#[reflect(Component, Default)]
pub struct CascadesFrusta {
    #[reflect(ignore)]
    pub frusta: EntityHashMap<Vec<Frustum>>,
}
