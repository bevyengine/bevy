use crate::{
    CascadeShadowConfig, Cascades, DirectionalLight, Material, PointLight, SpotLight,
    StandardMaterial,
};
use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::{Entity, EntityHashMap};
use bevy_ecs::{bundle::Bundle, component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    mesh::Mesh,
    primitives::{CascadesFrusta, CubemapFrusta, Frustum},
    view::{InheritedVisibility, ViewVisibility, Visibility},
};
use bevy_transform::components::{GlobalTransform, Transform};

/// A component bundle for PBR entities with a [`Mesh`] and a [`StandardMaterial`].
pub type PbrBundle = MaterialMesh3dBundle<StandardMaterial>;

/// A component bundle for entities with a [`Mesh3d`] and a [`MeshMaterial3d`].
#[derive(Bundle, Clone)]
pub struct MaterialMesh3dBundle<M: Material> {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<M>,
}

impl<M: Material> Default for MaterialMesh3dBundle<M> {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            material: Default::default(),
        }
    }
}

/// A component for rendering 3D meshes, typically with a [material] such as [`StandardMaterial`].
///
/// [material]: crate::material::Material
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(
    Transform,
    GlobalTransform,
    Visibility,
    InheritedVisibility,
    ViewVisibility
)]
pub struct Mesh3d(pub Handle<Mesh>);

impl From<Handle<Mesh>> for Mesh3d {
    fn from(handle: Handle<Mesh>) -> Self {
        Self(handle)
    }
}

/// A [material](Material) for a [`Mesh3d`](crate::Mesh3d).
#[derive(Component, Clone, Debug, Deref, DerefMut, PartialEq, Eq)]
pub struct MeshMaterial3d<M: Material>(pub Handle<M>);

impl<M: Material> Default for MeshMaterial3d<M> {
    fn default() -> Self {
        Self(Handle::default())
    }
}

impl<M: Material> From<Handle<M>> for MeshMaterial3d<M> {
    fn from(handle: Handle<M>) -> Self {
        Self(handle)
    }
}

/// Collection of mesh entities visible for 3D lighting.
/// This component contains all mesh entities visible from the current light view.
/// The collection is updated automatically by [`crate::SimulationLightSystems`].
#[derive(Component, Clone, Debug, Default, Reflect, Deref, DerefMut)]
#[reflect(Component)]
pub struct VisibleMeshEntities {
    #[reflect(ignore)]
    pub entities: Vec<Entity>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
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
#[reflect(Component)]
pub struct CascadesVisibleEntities {
    /// Map of view entity to the visible entities for each cascade frustum.
    #[reflect(ignore)]
    pub entities: EntityHashMap<Vec<VisibleMeshEntities>>,
}

/// A component bundle for [`PointLight`] entities.
#[derive(Debug, Bundle, Default, Clone)]
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
}

/// A component bundle for spot light entities
#[derive(Debug, Bundle, Default, Clone)]
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
}

/// A component bundle for [`DirectionalLight`] entities.
#[derive(Debug, Bundle, Default, Clone)]
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
}
