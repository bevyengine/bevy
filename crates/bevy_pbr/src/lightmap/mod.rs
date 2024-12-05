//! Lightmaps, baked lighting textures that can be applied at runtime to provide
//! diffuse global illumination.
//!
//! Bevy doesn't currently have any way to actually bake lightmaps, but they can
//! be baked in an external tool like [Blender](http://blender.org), for example
//! with an addon like [The Lightmapper]. The tools in the [`bevy-baked-gi`]
//! project support other lightmap baking methods.
//!
//! When a [`Lightmap`] component is added to an entity with a [`Mesh3d`] and a
//! [`MeshMaterial3d<StandardMaterial>`], Bevy applies the lightmap when rendering. The brightness
//! of the lightmap may be controlled with the `lightmap_exposure` field on
//! [`StandardMaterial`].
//!
//! During the rendering extraction phase, we extract all lightmaps into the
//! [`RenderLightmaps`] table, which lives in the render world. Mesh bindgroup
//! and mesh uniform creation consults this table to determine which lightmap to
//! supply to the shader. Essentially, the lightmap is a special type of texture
//! that is part of the mesh instance rather than part of the material (because
//! multiple meshes can share the same material, whereas sharing lightmaps is
//! nonsensical).
//!
//! Note that multiple meshes can't be drawn in a single drawcall if they use
//! different lightmap textures, unless bindless textures are in use. If you
//! want to instance a lightmapped mesh, and your platform doesn't support
//! bindless textures, combine the lightmap textures into a single atlas, and
//! set the `uv_rect` field on [`Lightmap`] appropriately.
//!
//! [The Lightmapper]: https://github.com/Naxela/The_Lightmapper
//! [`Mesh3d`]: bevy_render::mesh::Mesh3d
//! [`MeshMaterial3d<StandardMaterial>`]: crate::StandardMaterial
//! [`StandardMaterial`]: crate::StandardMaterial
//! [`bevy-baked-gi`]: https://github.com/pcwalton/bevy-baked-gi

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_math::{uvec2, vec4, Rect, UVec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    mesh::{Mesh, RenderMesh},
    render_asset::RenderAssets,
    render_resource::{Sampler, Shader, TextureView, WgpuSampler, WgpuTextureView},
    texture::{FallbackImage, GpuImage},
    view::ViewVisibility,
    Extract, ExtractSchedule, RenderApp,
};
use bevy_render::{renderer::RenderDevice, sync_world::MainEntityHashMap};
use bevy_utils::default;
use nonmax::NonMaxU32;

use crate::{binding_arrays_are_usable, ExtractMeshesSet, RenderMeshInstances};

/// The ID of the lightmap shader.
pub const LIGHTMAP_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(285484768317531991932943596447919767152);

/// The number of lightmaps that we store in a single slab, if bindless textures
/// are in use.
///
/// If bindless textures aren't in use, then only a single lightmap can be bound
/// at a time.
pub const LIGHTMAPS_PER_SLAB: usize = 16;

/// A plugin that provides an implementation of lightmaps.
pub struct LightmapPlugin;

/// A component that applies baked indirect diffuse global illumination from a
/// lightmap.
///
/// When assigned to an entity that contains a [`Mesh3d`](bevy_render::mesh::Mesh3d) and a
/// [`MeshMaterial3d<StandardMaterial>`](crate::StandardMaterial), if the mesh
/// has a second UV layer ([`ATTRIBUTE_UV_1`](bevy_render::mesh::Mesh::ATTRIBUTE_UV_1)),
/// then the lightmap will render using those UVs.
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

    /// The index of the slab (i.e. binding array) in which the lightmap is
    /// located.
    pub(crate) slab_index: LightmapSlabIndex,

    /// The index of the slot (i.e. element within the binding array) in which
    /// the lightmap is located.
    ///
    /// If bindless lightmaps aren't in use, this will be 0.
    pub(crate) slot_index: LightmapSlotIndex,
}

/// Stores data for all lightmaps in the render world.
///
/// This is cleared and repopulated each frame during the `extract_lightmaps`
/// system.
#[derive(Resource)]
pub struct RenderLightmaps {
    /// The mapping from every lightmapped entity to its lightmap info.
    ///
    /// Entities without lightmaps, or for which the mesh or lightmap isn't
    /// loaded, won't have entries in this table.
    pub(crate) render_lightmaps: MainEntityHashMap<RenderLightmap>,

    /// The slabs (binding arrays) containing the lightmaps.
    pub(crate) slabs: Vec<LightmapSlab>,

    /// Whether bindless textures are supported on this platform.
    pub(crate) bindless_supported: bool,
}

/// A binding array that contains lightmaps.
///
/// This will have a single binding if bindless lightmaps aren't in use.
#[derive(Default)]
pub struct LightmapSlab {
    /// The GPU images in this slab.
    gpu_images: Vec<GpuImage>,
}

/// The index of the slab (binding array) in which a lightmap is located.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deref, DerefMut)]
#[repr(transparent)]
pub struct LightmapSlabIndex(pub(crate) NonMaxU32);

/// The index of the slot (element within the binding array) in the slab in
/// which a lightmap is located.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deref, DerefMut)]
#[repr(transparent)]
pub(crate) struct LightmapSlotIndex(pub(crate) NonMaxU32);

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
    meshes: Res<RenderAssets<RenderMesh>>,
) {
    // Clear out the old frame's data.
    // TODO: Should we retain slabs from frame to frame, to avoid having to do
    // this?
    render_lightmaps.render_lightmaps.clear();
    render_lightmaps.slabs.clear();

    // Loop over each entity.
    for (entity, view_visibility, lightmap) in lightmaps.iter() {
        // Only process visible entities.
        if !view_visibility.get() {
            continue;
        }

        // Make sure the lightmap is loaded.
        let Some(gpu_image) = images.get(&lightmap.image) else {
            continue;
        };

        // Make sure the mesh is located and that it contains a lightmap UV map.
        if !render_mesh_instances
            .mesh_asset_id(entity.into())
            .and_then(|mesh_asset_id| meshes.get(mesh_asset_id))
            .is_some_and(|mesh| mesh.layout.0.contains(Mesh::ATTRIBUTE_UV_1.id))
        {
            continue;
        }

        // Add the lightmap to a slab.
        let (slab_index, slot_index) = render_lightmaps.add((*gpu_image).clone());

        // Store information about the lightmap in the render world.
        render_lightmaps.render_lightmaps.insert(
            entity.into(),
            RenderLightmap::new(
                lightmap.image.id(),
                lightmap.uv_rect,
                slab_index,
                slot_index,
            ),
        );
    }
}

impl RenderLightmap {
    /// Creates a new lightmap from a texture, a UV rect, and a slab and slot
    /// index pair.
    fn new(
        image: AssetId<Image>,
        uv_rect: Rect,
        slab_index: LightmapSlabIndex,
        slot_index: LightmapSlotIndex,
    ) -> Self {
        Self {
            image,
            uv_rect,
            slab_index,
            slot_index,
        }
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

impl FromWorld for RenderLightmaps {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bindless_supported = binding_arrays_are_usable(render_device);

        RenderLightmaps {
            render_lightmaps: default(),
            slabs: vec![],
            bindless_supported,
        }
    }
}

impl RenderLightmaps {
    /// Returns true if the slab with the given index is full or false otherwise.
    ///
    /// The slab must exist.
    fn slab_is_full(&self, slab_index: LightmapSlabIndex) -> bool {
        let size = self.slabs[u32::from(slab_index.0) as usize]
            .gpu_images
            .len();
        if self.bindless_supported {
            size >= LIGHTMAPS_PER_SLAB
        } else {
            size >= 1
        }
    }

    /// Creates a new slab, appends it to the end of the list, and returns its
    /// slab index.
    fn create_slab(&mut self) -> LightmapSlabIndex {
        let slab_index = LightmapSlabIndex(NonMaxU32::new(self.slabs.len() as u32).unwrap());
        self.slabs.push(default());
        slab_index
    }

    /// Adds a lightmap to a slab and returns the index of that slab as well as
    /// the index of the slot that the lightmap now occupies.
    ///
    /// This creates a new slab if there are no slabs or all slabs are full.
    fn add(&mut self, gpu_image: GpuImage) -> (LightmapSlabIndex, LightmapSlotIndex) {
        let mut slab_index = LightmapSlabIndex(NonMaxU32::new(self.slabs.len() as u32).unwrap());
        if (u32::from(*slab_index) as usize) >= self.slabs.len() || self.slab_is_full(slab_index) {
            slab_index = self.create_slab();
        }

        let slot_index = self.slabs[u32::from(*slab_index) as usize].insert(gpu_image);

        (slab_index, slot_index)
    }
}

impl LightmapSlab {
    /// Inserts a lightmap into this slab and returns the index of its slot.
    fn insert(&mut self, gpu_image: GpuImage) -> LightmapSlotIndex {
        let slot_index = LightmapSlotIndex(NonMaxU32::new(self.gpu_images.len() as u32).unwrap());
        self.gpu_images.push(gpu_image);
        slot_index
    }

    /// Returns the texture views and samplers for the lightmaps in this slab,
    /// ready to be placed into a bind group.
    ///
    /// This is used when constructing bind groups in bindless mode. Before
    /// returning, this function pads out the arrays with fallback images in
    /// order to fulfill requirements of platforms that require full binding
    /// arrays (e.g. DX12).
    pub(crate) fn build_binding_arrays(
        &mut self,
        fallback_images: &FallbackImage,
    ) -> (Vec<&WgpuTextureView>, Vec<&WgpuSampler>) {
        while self.gpu_images.len() < LIGHTMAPS_PER_SLAB {
            self.gpu_images.push(fallback_images.d2.clone());
        }
        (
            self.gpu_images
                .iter()
                .map(|gpu_image| &*gpu_image.texture_view)
                .collect(),
            self.gpu_images
                .iter()
                .map(|gpu_image| &*gpu_image.sampler)
                .collect(),
        )
    }

    /// Returns the texture view and sampler corresponding to the first
    /// lightmap, which must exist.
    ///
    /// This is used when constructing bind groups in non-bindless mode.
    pub(crate) fn bindings_for_first_lightmap(&self) -> (&TextureView, &Sampler) {
        (
            &self.gpu_images[0].texture_view,
            &self.gpu_images[0].sampler,
        )
    }
}
