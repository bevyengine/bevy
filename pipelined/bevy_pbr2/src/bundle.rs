use crate::{DirectionalLight, PointLight, StandardMaterial};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render2::{
    mesh::Mesh,
    view::{ComputedVisibility, Visibility, VisibleEntities},
};
use bevy_transform::components::{GlobalTransform, Transform};

#[derive(Bundle, Clone)]
pub struct PbrBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl Default for PbrBundle {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CubeFrustaVisibleEntities {
    cube_frusta_visible_entities: [VisibleEntities; 6],
}

impl CubeFrustaVisibleEntities {
    pub fn get(&self, i: usize) -> &VisibleEntities {
        &self.cube_frusta_visible_entities[i]
    }

    pub fn get_mut(&mut self, i: usize) -> &mut VisibleEntities {
        &mut self.cube_frusta_visible_entities[i]
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleEntities> {
        self.cube_frusta_visible_entities.iter()
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut VisibleEntities> {
        self.cube_frusta_visible_entities.iter_mut()
    }
}

/// A component bundle for "point light" entities
#[derive(Debug, Bundle, Default)]
pub struct PointLightBundle {
    pub point_light: PointLight,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

/// A component bundle for "directional light" entities
#[derive(Debug, Bundle, Default)]
pub struct DirectionalLightBundle {
    pub directional_light: DirectionalLight,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}
