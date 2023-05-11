use crate::{
    CascadeShadowConfig, Cascades, DirectionalLight, Material, PointLight, SpotLight,
    StandardMaterial,
};
use bevy_asset::Handle;
use bevy_ecs::{bundle::Bundle, component::Component, prelude::Entity, reflect::ReflectComponent};
use bevy_reflect::Reflect;
use bevy_render::{
    mesh::Mesh,
    primitives::{CascadesFrusta, CubemapFrusta, Frustum},
    view::{ComputedVisibility, Visibility, VisibleEntities},
};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_utils::HashMap;

/// A component bundle for PBR entities with a [`Mesh`] and a [`StandardMaterial`].
pub type PbrBundle = MaterialMeshBundle<StandardMaterial>;

/// A component bundle for entities with a [`Mesh`] and a [`Material`].
#[derive(Bundle, Clone)]
pub struct MaterialMeshBundle<M: Material> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<M>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl<M: Material> Default for MaterialMeshBundle<M> {
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

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CubemapVisibleEntities {
    #[reflect(ignore)]
    data: [VisibleEntities; 6],
}

impl CubemapVisibleEntities {
    pub fn get(&self, i: usize) -> &VisibleEntities {
        &self.data[i]
    }

    pub fn get_mut(&mut self, i: usize) -> &mut VisibleEntities {
        &mut self.data[i]
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleEntities> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut VisibleEntities> {
        self.data.iter_mut()
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct CascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore)]
    pub entities: HashMap<Entity, Vec<VisibleEntities>>,
}

/// A component bundle for [`PointLight`] entities.
#[derive(Debug, Bundle, Default)]
pub struct PointLightBundle {
    pub point_light: PointLight,
    pub cubemap_visible_entities: CubemapVisibleEntities,
    pub cubemap_frusta: CubemapFrusta,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// Enables or disables the light
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

/// A component bundle for spot light entities
#[derive(Debug, Bundle, Default)]
pub struct SpotLightBundle {
    pub spot_light: SpotLight,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// Enables or disables the light
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

/// A component bundle for [`DirectionalLight`] entities.
#[derive(Debug, Bundle, Default)]
pub struct DirectionalLightBundle {
    pub directional_light: DirectionalLight,
    pub frusta: CascadesFrusta,
    pub cascades: Cascades,
    pub cascade_shadow_config: CascadeShadowConfig,
    pub visible_entities: CascadesVisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// Enables or disables the light
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}
