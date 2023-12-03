//! Lightmaps, baked lighting textures that can be applied at runtime to provide
//! global illumination.
//!
//! Bevy doesn't currently have any way to actually bake lightmaps, but they can
//! be baked in an external tool like [Blender](http://blender.org), perhaps
//! with an addon like [The Lightmapper]. The tools in the [`bevy-baked-gi`]
//! project support other lightmap baking methods.
//!
//! [The Lightmapper]: https://github.com/Naxela/The_Lightmapper
//!
//! [`bevy-baked-gi`]: https://github.com/pcwalton/bevy-baked-gi

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{lifetimeless::Read, Query, Res, ResMut, Resource},
};
use bevy_math::Rect;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    mesh::Mesh,
    render_asset::RenderAssets,
    render_resource::Shader,
    texture::{GpuImage, Image},
    Render, RenderApp, RenderSet,
};
use bevy_utils::{
    hashbrown::{hash_map::Entry, HashMap},
    EntityHashMap,
};

use crate::{Mesh3d, RenderMeshInstances};

/// The maximum number of lightmaps in a scene.
pub const MAX_LIGHTMAPS: usize = 1024;

/// The ID of the lightmap shader.
pub const LIGHTMAP_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(285484768317531991932943596447919767152);

/// A plugin that provides an implementation of lightmaps.
pub struct LightmapPlugin;

/// A component that applies baked indirect global illumination from a lightmap.
///
/// When assigned to an entity that contains a [Mesh], if the mesh has a second
/// UV layer (UV1), then the lightmap will render using those UVs.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Lightmap {
    /// The lightmap image.
    pub image: Handle<Image>,

    /// The rectangle within the lightmap image that the UVs are relative to.
    ///
    /// The rect ranges from (0, 0) to (1, 1).
    ///
    /// This field allows lightmaps for a variety of meshes to be packed into a
    /// single atlas.
    pub uv_rect: Rect,
}

/// A render world resource that stores all lightmaps associated with each mesh.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct RenderLightmaps(pub HashMap<AssetId<Mesh>, RenderMeshLightmaps>);

/// Lightmaps associated with a mesh.
pub struct RenderMeshLightmaps {
    /// Maps an entity to lightmap index of the lightmap within
    /// `render_mesh_lightmaps`.
    pub(crate) entity_to_lightmap_index: EntityHashMap<Entity, RenderMeshLightmapIndex>,

    /// Maps a lightmap key to the index of the lightmap inside
    /// `render_mesh_lightmaps`.
    pub(crate) render_mesh_lightmap_to_lightmap_index:
        HashMap<RenderMeshLightmapKey, RenderMeshLightmapIndex>,

    /// A list of all (lightmap image, exposure) pairs used in the scene. Each
    /// element in this array should be unique.
    pub(crate) render_mesh_lightmaps: Vec<RenderMeshLightmap>,
}

/// A key that can be used to fetch a [`RenderMeshLightmap`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RenderMeshLightmapKey {
    /// The ID of the lightmap texture.
    pub(crate) image: AssetId<Image>,
}

/// Per-mesh data needed to render a lightmap.
///
/// The UV rect varies per mesh *instance*, not per mesh, so it's not stored
/// here.
pub struct RenderMeshLightmap {
    /// The lightmap texture.
    pub(crate) image: GpuImage,
    /// The ID of the lightmap's image asset.
    pub(crate) image_id: AssetId<Image>,
}

/// The index of a lightmap within the `render_mesh_lightmaps` array in
/// [`RenderMeshLightmaps`].
#[derive(Clone, Copy, Default, Deref)]
pub(crate) struct RenderMeshLightmapIndex(pub(crate) usize);

impl Plugin for LightmapPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            LIGHTMAP_SHADER_HANDLE,
            "lightmap.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(ExtractComponentPlugin::<Lightmap>::default())
            .register_type::<Lightmap>();

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<RenderLightmaps>().add_systems(
            Render,
            build_lightmap_texture_arrays.in_set(RenderSet::PrepareResources),
        );
    }
}

/// A system, part of the [`RenderApp`], that finds all lightmapped meshes in
/// the scene and updates the [`RenderMeshLightmaps`] resource.
///
/// This runs before batch building.
pub fn build_lightmap_texture_arrays(
    mut render_lightmaps: ResMut<RenderLightmaps>,
    lightmaps: Query<(Entity, &Lightmap), With<Mesh3d>>,
    images: Res<RenderAssets<Image>>,
    render_mesh_instances: Res<RenderMeshInstances>,
) {
    // Rebuild all lightmaps for this frame.
    render_lightmaps.clear();
    render_lightmaps.update(lightmaps, &render_mesh_instances, &images);
}

impl ExtractComponent for Lightmap {
    type Query = Read<Lightmap>;
    type Filter = ();
    type Out = Lightmap;

    fn extract_component(item: QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some((*item).clone())
    }
}

impl Default for Lightmap {
    fn default() -> Self {
        Self {
            image: Default::default(),
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
        }
    }
}

impl RenderMeshLightmaps {
    fn new() -> RenderMeshLightmaps {
        RenderMeshLightmaps {
            entity_to_lightmap_index: EntityHashMap::default(),
            render_mesh_lightmap_to_lightmap_index: HashMap::new(),
            render_mesh_lightmaps: vec![],
        }
    }
}

impl RenderLightmaps {
    /// Gathers information about all the lightmaps needed in this scene.
    fn update(
        &mut self,
        lightmaps: Query<(Entity, &Lightmap), With<Mesh3d>>,
        render_mesh_instances: &RenderMeshInstances,
        images: &RenderAssets<Image>,
    ) {
        for (entity, lightmap) in lightmaps.iter() {
            // If the mesh isn't loaded, skip it.
            let Some(mesh_id) = render_mesh_instances.get(&entity) else {
                continue;
            };

            // If the lightmap hasn't loaded, skip it.
            let Some(image) = images.get(&lightmap.image) else {
                continue;
            };

            let render_mesh_lightmaps = self
                .entry(mesh_id.mesh_asset_id)
                .or_insert_with(RenderMeshLightmaps::new);

            let render_mesh_lightmap_key = RenderMeshLightmapKey::from(lightmap);

            // We might already have an entry in the list corresponding to this
            // lightmap texture and exposure value. This will frequently occur
            // if, for example, multiple meshes share the same lightmap texture.
            // We can share the lightmap data among all such meshes in that
            // case.
            let render_mesh_lightmap_index = match render_mesh_lightmaps
                .render_mesh_lightmap_to_lightmap_index
                .entry(render_mesh_lightmap_key)
            {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    // Make a new lightmap data record.
                    let index =
                        RenderMeshLightmapIndex(render_mesh_lightmaps.render_mesh_lightmaps.len());
                    render_mesh_lightmaps
                        .render_mesh_lightmaps
                        .push(RenderMeshLightmap::new(
                            (*image).clone(),
                            lightmap.image.id(),
                        ));
                    entry.insert(index);
                    index
                }
            };

            render_mesh_lightmaps
                .entity_to_lightmap_index
                .insert(entity, render_mesh_lightmap_index);
        }
    }

    pub(crate) fn lightmap_key_for_entity(
        &self,
        mesh_asset_id: AssetId<Mesh>,
        entity: Entity,
    ) -> Option<RenderMeshLightmapKey> {
        let render_mesh_lightmaps = self.get(&mesh_asset_id)?;
        let lightmap_index = render_mesh_lightmaps
            .entity_to_lightmap_index
            .get(&entity)?;
        Some((&render_mesh_lightmaps.render_mesh_lightmaps[lightmap_index.0]).into())
    }
}

impl RenderMeshLightmap {
    fn new(image: GpuImage, image_id: AssetId<Image>) -> Self {
        Self { image, image_id }
    }
}

impl<'a> From<&'a Lightmap> for RenderMeshLightmapKey {
    fn from(lightmap: &'a Lightmap) -> Self {
        RenderMeshLightmapKey {
            image: lightmap.image.id(),
        }
    }
}

impl<'a> From<&'a RenderMeshLightmap> for RenderMeshLightmapKey {
    fn from(lightmap: &'a RenderMeshLightmap) -> Self {
        RenderMeshLightmapKey {
            image: lightmap.image_id,
        }
    }
}
