//! Lightmaps, baked lighting textures that can be applied at runtime to provide
//! diffuse global illumination.
//!
//! Bevy doesn't currently have any way to actually bake lightmaps, but they can
//! be baked in an external tool like [Blender](http://blender.org), for example
//! with an addon like [The Lightmapper]. The tools in the [`bevy-baked-gi`]
//! project support other lightmap baking methods.
//!
//! When a [`Lightmap`] component is added to an entity with a [`Mesh`] and a
//! [`StandardMaterial`](crate::StandardMaterial), Bevy applies the lightmap when rendering. The brightness
//! of the lightmap may be controlled with the `lightmap_exposure` field on
//! `StandardMaterial`.
//!
//! During the rendering extraction phase, we extract all lightmaps into the
//! [`RenderLightmaps`] table, which lives in the render world. Mesh bindgroup
//! and mesh uniform creation consults this table to determine which lightmap to
//! supply to the shader. Essentially, the lightmap is a special type of texture
//! that is part of the mesh instance rather than part of the material (because
//! multiple meshes can share the same material, whereas sharing lightmaps is
//! nonsensical).
//!
//! Note that meshes can't be instanced if they use different lightmap textures.
//! If you want to instance a lightmapped mesh, combine the lightmap textures
//! into a single atlas, and set the `uv_rect` field on [`Lightmap`]
//! appropriately.
//!
//! [The Lightmapper]: https://github.com/Naxela/The_Lightmapper
//!
//! [`bevy-baked-gi`]: https://github.com/pcwalton/bevy-baked-gi

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut, Resource},
};
use bevy_math::{uvec2, vec4, Rect, UVec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::mesh::GpuMesh;
use bevy_render::texture::GpuImage;
use bevy_render::{
    mesh::Mesh, render_asset::RenderAssets, render_resource::Shader, texture::Image,
    view::ViewVisibility, Extract, ExtractSchedule, RenderApp,
};
use bevy_utils::HashSet;

use crate::{ExtractMeshesSet, RenderMeshInstances};

/// The ID of the lightmap shader.
pub const LIGHTMAP_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(285484768317531991932943596447919767152);

/// A plugin that provides an implementation of lightmaps.
pub struct LightmapPlugin;

/// A component that applies baked indirect diffuse global illumination from a
/// lightmap.
///
/// When assigned to an entity that contains a [`Mesh`] and a
/// [`StandardMaterial`](crate::StandardMaterial), if the mesh has a second UV
/// layer ([`ATTRIBUTE_UV_1`](bevy_render::mesh::Mesh::ATTRIBUTE_UV_1)), then
/// the lightmap will render using those UVs.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Lightmap {
    /// The lightmap texture.
    pub image: Handle<Image>,

    /// The rectangle within the lightmap texture that the UVs are relative to.
    ///
    /// The top left coordinate is the `min` part of the rect, and the bottom
    /// right coordinate is the `max` part of the rect. The rect ranges from (0,
    /// 0) to (1, 1).
    ///
    /// This field allows lightmaps for a variety of meshes to be packed into a
    /// single atlas.
    pub uv_rect: Rect,
}

/// Lightmap data stored in the render world.
///
/// There is one of these per visible lightmapped mesh instance.
#[derive(Debug)]
pub(crate) struct RenderLightmap {
    /// The ID of the lightmap texture.
    pub(crate) image: AssetId<Image>,

    /// The rectangle within the lightmap texture that the UVs are relative to.
    ///
    /// The top left coordinate is the `min` part of the rect, and the bottom
    /// right coordinate is the `max` part of the rect. The rect ranges from (0,
    /// 0) to (1, 1).
    pub(crate) uv_rect: Rect,
}

/// Stores data for all lightmaps in the render world.
///
/// This is cleared and repopulated each frame during the `extract_lightmaps`
/// system.
#[derive(Default, Resource)]
pub struct RenderLightmaps {
    /// The mapping from every lightmapped entity to its lightmap info.
    ///
    /// Entities without lightmaps, or for which the mesh or lightmap isn't
    /// loaded, won't have entries in this table.
    pub(crate) render_lightmaps: EntityHashMap<RenderLightmap>,

    /// All active lightmap images in the scene.
    ///
    /// Gathering all lightmap images into a set makes mesh bindgroup
    /// preparation slightly more efficient, because only one bindgroup needs to
    /// be created per lightmap texture.
    pub(crate) all_lightmap_images: HashSet<AssetId<Image>>,
}

impl Plugin for LightmapPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            LIGHTMAP_SHADER_HANDLE,
            "lightmap.wgsl",
            Shader::from_wgsl
        );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderLightmaps>()
            .add_systems(ExtractSchedule, extract_lightmaps.after(ExtractMeshesSet));
    }
}

/// Extracts all lightmaps from the scene and populates the [`RenderLightmaps`]
/// resource.
fn extract_lightmaps(
    mut render_lightmaps: ResMut<RenderLightmaps>,
    lightmaps: Extract<Query<(Entity, &ViewVisibility, &Lightmap)>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    images: Res<RenderAssets<GpuImage>>,
    meshes: Res<RenderAssets<GpuMesh>>,
) {
    // Clear out the old frame's data.
    render_lightmaps.render_lightmaps.clear();
    render_lightmaps.all_lightmap_images.clear();

    // Loop over each entity.
    for (entity, view_visibility, lightmap) in lightmaps.iter() {
        // Only process visible entities for which the mesh and lightmap are
        // both loaded.
        if !view_visibility.get()
            || images.get(&lightmap.image).is_none()
            || !render_mesh_instances
                .mesh_asset_id(entity)
                .and_then(|mesh_asset_id| meshes.get(mesh_asset_id))
                .is_some_and(|mesh| mesh.layout.0.contains(Mesh::ATTRIBUTE_UV_1.id))
        {
            continue;
        }

        // Store information about the lightmap in the render world.
        render_lightmaps.render_lightmaps.insert(
            entity,
            RenderLightmap::new(lightmap.image.id(), lightmap.uv_rect),
        );

        // Make a note of the loaded lightmap image so we can efficiently
        // process them later during mesh bindgroup creation.
        render_lightmaps
            .all_lightmap_images
            .insert(lightmap.image.id());
    }
}

impl RenderLightmap {
    /// Creates a new lightmap from a texture and a UV rect.
    fn new(image: AssetId<Image>, uv_rect: Rect) -> Self {
        Self { image, uv_rect }
    }
}

/// Packs the lightmap UV rect into 64 bits (4 16-bit unsigned integers).
pub(crate) fn pack_lightmap_uv_rect(maybe_rect: Option<Rect>) -> UVec2 {
    match maybe_rect {
        Some(rect) => {
            let rect_uvec4 = (vec4(rect.min.x, rect.min.y, rect.max.x, rect.max.y) * 65535.0)
                .round()
                .as_uvec4();
            uvec2(
                rect_uvec4.x | (rect_uvec4.y << 16),
                rect_uvec4.z | (rect_uvec4.w << 16),
            )
        }
        None => UVec2::ZERO,
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
