//! Lightmaps, baked lighting textures that can be applied at runtime to provide
//! global illumination.
//!
//! Bevy doesn't currently have any way to actually bake lightmaps, but they can
//! be baked in an external tool like [Blender](http://blender.org), perhaps
//! with an addon like [The Lightmapper].
//!
//! When a mesh is instanced, each instance typically needs a separate lightmap.
//! In such circumstances, this plugin combines all of the mesh's lightmaps into
//! a single texture array. To do this, it requires that all lightmaps attached
//! to a mesh have the same size and texture format. A warning will be reported
//! and some lightmaps will be ignored if this restriction isn't followed.
//!
//! [The Lightmapper]: https://github.com/Naxela/The_Lightmapper

use bevy_app::{App, Plugin};
use bevy_asset::{AssetId, Handle};
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
    render_resource::{ShaderType, UniformBuffer},
    renderer::{RenderDevice, RenderQueue},
    texture::{GpuImage, Image},
    Render, RenderApp, RenderSet,
};
use bevy_utils::{
    hashbrown::{hash_map::Entry, HashMap},
    EntityHashMap, FloatOrd,
};
use bytemuck::{Pod, Zeroable};

use crate::{Mesh3d, RenderMeshInstances};

/// The maximum number of lightmaps in a scene.
pub const MAX_LIGHTMAPS: usize = 1024;

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

    /// The intensity or brightness of the lightmap.
    ///
    /// Colors within the lightmap are multiplied by this value when rendering.
    pub exposure: f32,
}

/// The on-GPU structure that specifies metadata about the lightmap.
///
/// Currently, the only such metadata is the exposure
#[derive(ShaderType, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct GpuLightmap {
    /// The intensity or brightness of the lightmap.
    ///
    /// Colors within the lightmap are multiplied by this value when rendering.
    pub exposure: f32,
}

/// A render world resource that stores all lightmaps associated with each mesh.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct RenderLightmaps(pub HashMap<AssetId<Mesh>, RenderMeshLightmaps>);

/// Lightmaps associated with a mesh.
pub struct RenderMeshLightmaps {
    /// Maps an entity to lightmap index of the lightmap within
    /// `render_mesh_lightmaps`.
    entity_to_lightmap_index: EntityHashMap<Entity, RenderMeshLightmapIndex>,

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
    /// The exposure (brightness) value.
    pub(crate) exposure: FloatOrd,
}

/// Per-mesh data needed to render a lightmap.
///
/// The UV rect varies per mesh *instance*, not per mesh, so it's not stored
/// here.
pub struct RenderMeshLightmap {
    /// The lightmap texture.
    pub(crate) image: GpuImage,
    /// The exposure (brightness) value.
    pub(crate) exposure: f32,
}

/// The index of a lightmap within the `render_mesh_lightmaps` array in
/// [`RenderMeshLightmaps`].
#[derive(Clone, Copy, Default, Deref)]
pub(crate) struct RenderMeshLightmapIndex(pub(crate) usize);

/// GPU data that contains metadata about each lightmap.
///
/// Currently, the only metadata stored per lightmap is the exposure
/// (brightness) value.
#[derive(Resource, Default)]
pub struct LightmapUniforms {
    /// The GPU buffers containing metadata about each lightmap.
    pub exposure_to_lightmap_uniform: HashMap<FloatOrd, UniformBuffer<GpuLightmap>>,

    /// A fallback buffer used when the lightmap hasn't loaded yet.
    pub fallback_uniform: Option<UniformBuffer<GpuLightmap>>,
}

impl Plugin for LightmapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<Lightmap>::default())
            .register_type::<Lightmap>();

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderLightmaps>()
            .init_resource::<LightmapUniforms>()
            .add_systems(
                Render,
                (
                    build_lightmap_texture_arrays.in_set(RenderSet::PrepareResources),
                    upload_lightmaps_buffers
                        .in_set(RenderSet::PrepareResources)
                        .after(build_lightmap_texture_arrays),
                ),
            );
    }
}

/// A system, part of the [`RenderApp`], that finds all lightmapped meshes in
/// the scene and updates the [`RenderMeshLightmaps`] and [`LightmapUniforms`]
/// resources.
///
/// This runs before batch building.
pub fn build_lightmap_texture_arrays(
    mut render_lightmaps: ResMut<RenderLightmaps>,
    lightmaps: Query<(Entity, &Lightmap), With<Mesh3d>>,
    mut gpu_lightmaps: ResMut<LightmapUniforms>,
    images: Res<RenderAssets<Image>>,
    render_mesh_instances: Res<RenderMeshInstances>,
) {
    // Rebuild all lightmaps for this frame.
    render_lightmaps.clear();
    render_lightmaps.update(lightmaps, &render_mesh_instances, &images);

    // Update the lightmap metadata.
    gpu_lightmaps.update(&mut render_lightmaps);
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
            exposure: 1.0,
        }
    }
}

/// Uploads the lightmap metadata uniforms to the GPU.
pub fn upload_lightmaps_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniform: ResMut<LightmapUniforms>,
) {
    for buffer in uniform.exposure_to_lightmap_uniform.values_mut() {
        buffer.write_buffer(&render_device, &render_queue);
    }

    if let Some(ref mut buffer) = uniform.fallback_uniform {
        buffer.write_buffer(&render_device, &render_queue);
    }
}

impl Default for GpuLightmap {
    fn default() -> Self {
        Self { exposure: 1.0 }
    }
}

impl LightmapUniforms {
    // Prepares the uniform buffers containing lightmap metadata for this frame.
    //
    // Currently, the uniform buffers only contain exposure (brightness)
    // information.
    fn update(&mut self, render_lightmaps: &mut RenderLightmaps) {
        // Prefer to reuse buffers from a previous frame if possible.
        let mut spare_buffers: Vec<_> = self
            .exposure_to_lightmap_uniform
            .drain()
            .map(|(_, buffer)| buffer)
            .collect();

        for render_mesh_lightmaps in render_lightmaps.values() {
            for render_mesh_lightmap in &render_mesh_lightmaps.render_mesh_lightmaps {
                // If we already have an entry for that exposure value, skip it.
                let Entry::Vacant(entry) = self
                    .exposure_to_lightmap_uniform
                    .entry(FloatOrd(render_mesh_lightmap.exposure))
                else {
                    continue;
                };

                // Create a new buffer containing this exposure value.

                let gpu_lightmap = GpuLightmap {
                    exposure: render_mesh_lightmap.exposure,
                };

                let buffer = match spare_buffers.pop() {
                    Some(mut buffer) => {
                        buffer.set(gpu_lightmap);
                        buffer
                    }
                    None => gpu_lightmap.into(),
                };

                entry.insert(buffer);
            }
        }

        // Build a fallback uniform, for use when lightmaps haven't loaded yet.
        if self.fallback_uniform.is_none() {
            self.fallback_uniform = Some(GpuLightmap::default().into());
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
                .or_insert_with(|| RenderMeshLightmaps::new());

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
                        .push(RenderMeshLightmap::new((*image).clone(), lightmap.exposure));
                    entry.insert(index);
                    index
                }
            };

            render_mesh_lightmaps
                .entity_to_lightmap_index
                .insert(entity, render_mesh_lightmap_index);
        }
    }
}

impl RenderMeshLightmap {
    fn new(image: GpuImage, exposure: f32) -> Self {
        Self { image, exposure }
    }
}

impl<'a> From<&'a Lightmap> for RenderMeshLightmapKey {
    fn from(lightmap: &'a Lightmap) -> Self {
        RenderMeshLightmapKey {
            image: lightmap.image.id(),
            exposure: FloatOrd(lightmap.exposure),
        }
    }
}
