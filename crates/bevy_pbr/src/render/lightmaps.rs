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

    /// The intensity of the lightmap.
    ///
    /// Colors within the lightmap are multiplied by this value when rendering.
    pub exposure: f32,
}

/// The on-GPU structure that specifies various metadata about the lightmap.
#[derive(ShaderType, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct GpuLightmap {
    /// The intensity of the lightmap.
    pub exposure: f32,
}

/// A render world resource that stores all lightmaps associated with each mesh.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct RenderLightmaps(HashMap<AssetId<Mesh>, RenderMeshLightmaps>);

/// Lightmaps associated with a mesh.
pub struct RenderMeshLightmaps {
    entity_to_lightmap_index: EntityHashMap<Entity, RenderMeshLightmapIndex>,
    pub(crate) render_mesh_lightmap_to_lightmap_index:
        HashMap<RenderMeshLightmapKey, RenderMeshLightmapIndex>,
    pub(crate) render_mesh_lightmaps: Vec<RenderMeshLightmap>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RenderMeshLightmapKey {
    pub(crate) image: AssetId<Image>,
    pub(crate) exposure: FloatOrd,
}

pub struct RenderMeshLightmap {
    pub(crate) image: GpuImage,
    pub(crate) exposure: f32,
}

#[derive(Clone, Copy, Default, Deref)]
pub(crate) struct RenderMeshLightmapIndex(pub(crate) usize);

/// The index in the texture array of lightmaps for a particular mesh instance.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LightmapTextureArrayIndex {
    /// No array index was assigned because this lightmap was incompatible with
    /// other lightmaps for this mesh.
    ///
    /// We record this fact to avoid detecting such lightmaps as dirty every frame, which would
    /// otherwise cause us to try to reupload them every frame.
    Invalid,

    /// The lightmap has a valid texture array index.
    Valid(u32),
}

/// Holds the GPU structure containing metadata about each lightmap.
#[derive(Resource, Default)]
pub struct LightmapUniforms {
    /// The GPU buffers containing metadata about each lightmap.
    pub exposure_to_lightmap_uniform: HashMap<FloatOrd, UniformBuffer<GpuLightmap>>,
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
/// the scene, updates the [`LightmapUniform`], and combines the lightmaps into
/// texture arrays so they can be efficiently rendered.
pub fn build_lightmap_texture_arrays(
    mut render_lightmaps: ResMut<RenderLightmaps>,
    lightmaps: Query<(Entity, &Lightmap), With<Mesh3d>>,
    mut gpu_lightmaps: ResMut<LightmapUniforms>,
    images: Res<RenderAssets<Image>>,
    render_mesh_instances: Res<RenderMeshInstances>,
) {
    render_lightmaps.clear();

    for (entity, lightmap) in lightmaps.iter() {
        let Some(mesh_id) = render_mesh_instances.get(&entity) else {
            continue;
        };

        let Some(image) = images.get(&lightmap.image) else {
            continue;
        };

        let render_mesh_lightmaps = render_lightmaps
            .entry(mesh_id.mesh_asset_id)
            .or_insert_with(|| RenderMeshLightmaps::new());

        let render_mesh_lightmap_key = RenderMeshLightmapKey::from(lightmap);

        let render_mesh_lightmap_index = match render_mesh_lightmaps
            .render_mesh_lightmap_to_lightmap_index
            .entry(render_mesh_lightmap_key)
        {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
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
    // Updates the uniform buffer containing the lightmap metadata.
    //
    // This is called even if there are no texture updates because there could
    // be, for example, new mesh instances that are added to the world
    // referencing lightmap textures that are already uploaded to the GPU.
    fn update(&mut self, render_lightmaps: &mut RenderLightmaps) {
        let mut spare_buffers: Vec<_> = self
            .exposure_to_lightmap_uniform
            .drain()
            .map(|(_, buffer)| buffer)
            .collect();

        for render_mesh_lightmaps in render_lightmaps.values() {
            for render_mesh_lightmap in &render_mesh_lightmaps.render_mesh_lightmaps {
                let Entry::Vacant(entry) = self
                    .exposure_to_lightmap_uniform
                    .entry(FloatOrd(render_mesh_lightmap.exposure))
                else {
                    continue;
                };

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
