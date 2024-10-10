#![expect(deprecated)]

use crate::{
    CascadeShadowConfig, Cascades, DirectionalLight, Material, MeshMaterial3d, PointLight,
    SpotLight, StandardMaterial,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::{Entity, EntityHashMap},
    reflect::ReflectComponent,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::sync_world::MainEntity;
use bevy_render::{
    mesh::Mesh3d,
    primitives::{CascadesFrusta, CubemapFrusta, Frustum},
    sync_world::SyncToRenderWorld,
    view::{InheritedVisibility, ViewVisibility, Visibility},
};
use bevy_transform::components::{GlobalTransform, Transform};

/// A component bundle for PBR entities with a [`Mesh3d`] and a [`MeshMaterial3d<StandardMaterial>`].
#[deprecated(
    since = "0.15.0",
    note = "Use the `Mesh3d` and `MeshMaterial3d` components instead. Inserting them will now also insert the other components required by them automatically."
)]
pub type PbrBundle = MaterialMeshBundle<StandardMaterial>;

/// A component bundle for entities with a [`Mesh3d`] and a [`MeshMaterial3d`].
#[derive(Bundle, Clone)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `Mesh3d` and `MeshMaterial3d` components instead. Inserting them will now also insert the other components required by them automatically."
)]
pub struct MaterialMeshBundle<M: Material> {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<M>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}

impl<M: Material> Default for MaterialMeshBundle<M> {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
        }
    }
}

/// Collection of mesh entities visible for 3D lighting.
///
/// This component contains all mesh entities visible from the current light view.
/// The collection is updated automatically by [`crate::SimulationLightSystems`].
#[derive(Component, Clone, Debug, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Debug, Default)]
pub struct VisibleMeshEntities {
    #[reflect(ignore)]
    pub entities: Vec<Entity>,
}

#[derive(Component, Clone, Debug, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Debug, Default)]
pub struct RenderVisibleMeshEntities {
    #[reflect(ignore)]
    pub entities: Vec<(Entity, MainEntity)>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct CubemapVisibleEntities {
    #[reflect(ignore)]
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
#[reflect(Component, Debug, Default)]
pub struct RenderCubemapVisibleEntities {
    #[reflect(ignore)]
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
#[reflect(Component)]
pub struct CascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore)]
    pub entities: EntityHashMap<Vec<VisibleMeshEntities>>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct RenderCascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore)]
    pub entities: EntityHashMap<Vec<RenderVisibleMeshEntities>>,
}

/// A component bundle for [`PointLight`] entities.
#[derive(Debug, Bundle, Default, Clone)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `PointLight` component instead. Inserting it will now also insert the other components required by it automatically."
)]
pub struct PointLightBundle {
    pub point_light: PointLight,
    pub cubemap_visible_entities: CubemapVisibleEntities,
    pub cubemap_frusta: CubemapFrusta,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// Enables or disables the light
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Marker component that indicates that its entity needs to be synchronized to the render world
    pub sync: SyncToRenderWorld,
}

/// A component bundle for spot light entities
#[derive(Debug, Bundle, Default, Clone)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `SpotLight` component instead. Inserting it will now also insert the other components required by it automatically."
)]
pub struct SpotLightBundle {
    pub spot_light: SpotLight,
    pub visible_entities: VisibleMeshEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// Enables or disables the light
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Marker component that indicates that its entity needs to be synchronized to the render world
    pub sync: SyncToRenderWorld,
}

/// A component bundle for [`DirectionalLight`] entities.
#[derive(Debug, Bundle, Default, Clone)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `DirectionalLight` component instead. Inserting it will now also insert the other components required by it automatically."
)]
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
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Marker component that indicates that its entity needs to be synchronized to the render world
    pub sync: SyncToRenderWorld,
}
