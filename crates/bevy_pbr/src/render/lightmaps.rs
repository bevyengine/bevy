//! Lightmaps, baked lighting textures that can be applied at runtime to provide
//! global illumination.
//!
//! Bevy doesn't currently have any way to actually bake lightmaps, but they can
//! be baked in an external tool like [Blender](http://blender.org), perhaps
//! with an addon like [The Lightmapper].
//!
//! [The Lightmapper]: https://github.com/Naxela/The_Lightmapper

use std::iter;

use bevy_app::{App, Plugin};
use bevy_asset::{AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::QueryItem,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{lifetimeless::Read, Query, Res, ResMut, Resource},
};
use bevy_math::{vec4, Rect, Vec2, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    mesh::Mesh,
    render_asset::RenderAssets,
    render_resource::{
        BufferUsages, BufferVec, CommandEncoder, CommandEncoderDescriptor, Extent3d, FilterMode,
        ImageCopyTexture, Origin3d, SamplerDescriptor, ShaderType, TextureAspect,
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
        TextureViewDimension,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::{GpuImage, Image},
    Render, RenderApp, RenderSet,
};
use bevy_utils::{
    hashbrown::hash_map::Entry,
    nonmax::NonMaxU32,
    tracing::{info, warn},
    EntityHashMap, HashMap,
};
use bytemuck::{Pod, Zeroable};

use crate::{Mesh3d, RenderMeshInstances};

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
}

/// The on-GPU structure that specifies various metadata about the lightmap.
#[derive(ShaderType, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct GpuLightmap {
    /// The UV rectangle within the lightmap.
    ///
    /// This is the same as `uv_rect` in the [Lightmap] component.
    pub uv_rect: Vec4,
    /// The texture array index.
    pub texture_array_index: u32,
    /// Unused GPU padding needed to pad out the structure.
    pub padding: [u32; 3],
}

/// A render world resource that stores all lightmaps associated with each mesh.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct RenderLightmaps(HashMap<AssetId<Mesh>, RenderLightmap>);

/// A lightmap associated with a mesh.
pub struct RenderLightmap {
    /// The [GpuImage] representing the lightmap.
    pub image: GpuImage,
    /// The index of the lightmap in this mesh's lightmap texture array.
    array_indices: HashMap<AssetId<Image>, LightmapTextureArrayIndex>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum LightmapTextureArrayIndex {
    // No array index was assigned because this lightmap was incompatible with
    // other lightmaps for this mesh.
    //
    // We record this fact to avoid detecting such lightmaps as dirty every frame, which would
    // otherwise cause us to try to reupload them every frame.
    Invalid,
    // The lightmap has a valid texture array index.
    Valid(u32),
}

/// Holds the GPU structure containing metadata about each lightmap.
#[derive(Resource)]
pub struct LightmapUniform {
    /// The GPU buffer containing metadata about each lightmap.
    pub buffer: BufferVec<GpuLightmap>,
    /// The index within `buffer` of each entity that has a lightmap.
    pub uniform_indices: EntityHashMap<Entity, NonMaxU32>,
}

// Information about a lightmap that we need to regenerate.
struct RenderLightmapDescriptor {
    // The size of the lightmap.
    image_size: Vec2,
    // The texture format of the lightmap.
    image_format: TextureFormat,
    /// The index of the lightmap in this mesh's lightmap texture array.
    array_indices: HashMap<AssetId<Image>, LightmapTextureArrayIndex>,
}

// Information about lightmaps for meshes that we need to regenerate.
#[derive(Deref, DerefMut)]
struct RenderLightmapDescriptors(HashMap<AssetId<Mesh>, RenderLightmapDescriptor>);

impl Plugin for LightmapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<Lightmap>::default())
            .register_type::<Lightmap>();

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<RenderLightmaps>()
            .init_resource::<LightmapUniform>()
            .add_systems(
                Render,
                (
                    build_lightmap_texture_arrays.in_set(RenderSet::PrepareResources),
                    upload_lightmaps_buffer
                        .in_set(RenderSet::PrepareResources)
                        .after(build_lightmap_texture_arrays),
                ),
            );
    }
}

/// A system, part of the [RenderApp], that finds all lightmapped meshes in the
/// scene, updates the [LightmapUniform], and combines the lightmaps into
/// texture arrays so they can be efficiently rendered.
pub fn build_lightmap_texture_arrays(
    mut render_lightmaps: ResMut<RenderLightmaps>,
    lightmaps: Query<(Entity, &Lightmap, &Mesh3d)>,
    mut gpu_lightmaps: ResMut<LightmapUniform>,
    images: Res<RenderAssets<Image>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // If there are no lightmaps in the scene, bail out.
    if lightmaps.is_empty() {
        return;
    }

    // Invalidate all out-of-date lightmaps.
    render_lightmaps.invalidate_lightmaps(&lightmaps, &render_mesh_instances, &images);

    // Build up a list of the new lightmaps to upload this frame.
    let mut new_lightmap_descriptors = RenderLightmapDescriptors::new();
    new_lightmap_descriptors.update_invalid_lightmaps(
        &mut render_lightmaps,
        &lightmaps,
        &render_mesh_instances,
        &images,
    );

    // Update the lightmap metadata.
    gpu_lightmaps.update(
        &new_lightmap_descriptors,
        &mut render_lightmaps,
        &lightmaps,
        &render_mesh_instances,
        &images,
    );

    // Upload new lightmap textures if necessary.
    new_lightmap_descriptors.create_and_copy_in_lightmaps(
        &mut render_lightmaps,
        &images,
        &render_device,
        &render_queue,
    );
}

impl RenderLightmaps {
    // Invalidates all out-of-date lightmaps.
    fn invalidate_lightmaps(
        &mut self,
        lightmaps: &Query<(Entity, &Lightmap, &Mesh3d)>,
        render_mesh_instances: &RenderMeshInstances,
        images: &RenderAssets<Image>,
    ) {
        for (entity, ref lightmap, _) in lightmaps.iter() {
            let Some(mesh_instance) = render_mesh_instances.get(&entity) else {
                continue;
            };
            if images.get(&lightmap.image).is_none() {
                continue;
            }

            if let Entry::Occupied(entry) = self.entry(mesh_instance.mesh_asset_id) {
                if !entry.get().array_indices.contains_key(&lightmap.image.id()) {
                    entry.remove_entry();
                }
            }
        }
    }
}

impl RenderLightmapDescriptors {
    // Creates a new empty set of render lightmap descriptors.
    fn new() -> Self {
        Self(HashMap::new())
    }

    // Finds all lightmaps that need updating and update them.
    fn update_invalid_lightmaps(
        &mut self,
        render_lightmaps: &mut RenderLightmaps,
        lightmaps: &Query<(Entity, &Lightmap, &Mesh3d)>,
        render_mesh_instances: &RenderMeshInstances,
        images: &RenderAssets<Image>,
    ) {
        for (entity, ref lightmap, _) in lightmaps.iter() {
            let (Some(mesh_instance), Some(gpu_lightmap_image)) = (
                render_mesh_instances.get(&entity),
                images.get(&lightmap.image),
            ) else {
                continue;
            };

            if render_lightmaps.contains_key(&mesh_instance.mesh_asset_id) {
                continue;
            }

            // Initialize a new lightmap if necessary.
            let lightmap_descriptor =
                self.entry(mesh_instance.mesh_asset_id).or_insert_with(|| {
                    RenderLightmapDescriptor {
                        image_size: gpu_lightmap_image.size,
                        image_format: gpu_lightmap_image.texture_format,
                        array_indices: HashMap::new(),
                    }
                });

            if lightmap_descriptor
                .array_indices
                .contains_key(&lightmap.image.id())
            {
                continue;
            }

            // Create a new lightmap array slice if necessary.
            if gpu_lightmap_image.size != lightmap_descriptor.image_size
                || gpu_lightmap_image.texture_format != lightmap_descriptor.image_format
            {
                // TODO: Better warning message.
                warn!("Ignoring lightmap {:?} because it was incompatible with the existing lightmap(s) for that mesh", gpu_lightmap_image);
                lightmap_descriptor
                    .array_indices
                    .insert(lightmap.image.id(), LightmapTextureArrayIndex::Invalid);
            } else {
                let array_index = LightmapTextureArrayIndex::Valid(
                    lightmap_descriptor.array_indices.len() as u32,
                );
                lightmap_descriptor
                    .array_indices
                    .insert(lightmap.image.id(), array_index);
            }
        }
    }

    // Creates a new lightmap array textures and fills it.
    fn create_and_copy_in_lightmaps(
        self,
        render_lightmaps: &mut RenderLightmaps,
        images: &RenderAssets<Image>,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
    ) {
        if self.is_empty() {
            return;
        }

        info!("Uploading {} new lightmap(s)", self.len());

        // Build a command encoder.
        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("copy_lightmaps"),
        });

        // For each mesh, create a texture array consisting of all lightmaps for that mesh.
        for (mesh_id, lightmap_descriptor) in self.0 {
            // Create the texture array, and copy in the lightmaps.
            //
            // TODO(pcwalton): Skip copying if there's only one lightmap texture?
            let lightmap_image = lightmap_descriptor.create_lightmap_texture_array(&render_device);
            lightmap_descriptor.copy_lightmaps_to_texture(
                &mut command_encoder,
                &lightmap_image,
                &images,
            );

            // Write the lightmap in.
            render_lightmaps.insert(
                mesh_id,
                RenderLightmap {
                    image: lightmap_image,
                    array_indices: lightmap_descriptor.array_indices,
                },
            );
        }

        // Submit the buffer.
        let command_buffer = command_encoder.finish();
        render_queue.submit(iter::once(command_buffer));
    }
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

impl Default for LightmapUniform {
    fn default() -> Self {
        Self {
            buffer: BufferVec::new(BufferUsages::UNIFORM),
            uniform_indices: EntityHashMap::default(),
        }
    }
}

/// Uploads the lightmap metadata uniform to the GPU.
pub fn upload_lightmaps_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniform: ResMut<LightmapUniform>,
) {
    let length = uniform.buffer.len();
    uniform.buffer.reserve(length, &render_device);
    uniform.buffer.write_buffer(&render_device, &render_queue);
}

impl Default for GpuLightmap {
    fn default() -> Self {
        Self {
            uv_rect: vec4(0.0, 0.0, 1.0, 1.0),
            texture_array_index: u32::MAX,
            padding: [0; 3],
        }
    }
}

impl LightmapUniform {
    // Updates the uniform buffer containing the lightmap metadata.
    //
    // This is called even if there are no texture updates because there could
    // be, for example, new mesh instances that are added to the world
    // referencing lightmap textures that are already uploaded to the GPU.
    fn update(
        &mut self,
        new_lightmap_descriptors: &RenderLightmapDescriptors,
        render_lightmaps: &mut RenderLightmaps,
        lightmaps: &Query<(Entity, &Lightmap, &Mesh3d)>,
        render_mesh_instances: &RenderMeshInstances,
        images: &RenderAssets<Image>,
    ) {
        // Reset the buffer.
        self.buffer.clear();
        self.uniform_indices.clear();

        // Build the metadata for each lightmap.
        for (entity, ref lightmap, _) in lightmaps.iter() {
            let Some(mesh_instance) = render_mesh_instances.get(&entity) else {
                continue;
            };
            if images.get(&lightmap.image).is_none() {
                continue;
            }

            // Find the texture array index for this lightmap. It'll either be
            // in our existing [RenderLightmaps] array or else in
            // `new_lightmap_descriptors` if it's new.
            let texture_array_index = match render_lightmaps.get(&mesh_instance.mesh_asset_id) {
                Some(render_lightmap) => render_lightmap.array_indices[&lightmap.image.id()],
                None => {
                    new_lightmap_descriptors[&mesh_instance.mesh_asset_id].array_indices
                        [&lightmap.image.id()]
                }
            };

            // Add the metadata entry.
            let lightmap_uniform_index = NonMaxU32::try_from(
                self.buffer.push(GpuLightmap {
                    texture_array_index: match texture_array_index {
                        LightmapTextureArrayIndex::Invalid => u32::MAX,
                        LightmapTextureArrayIndex::Valid(index) => index,
                    },
                    uv_rect: lightmap
                        .uv_rect
                        .min
                        .extend(lightmap.uv_rect.max.x)
                        .extend(lightmap.uv_rect.max.y),
                    padding: [0; 3],
                }) as u32,
            )
            .unwrap();

            // Record the index of the metadata.
            self.uniform_indices.insert(entity, lightmap_uniform_index);
        }

        // Pad out the array to the right length.
        let gpu_lightmap_count = self.buffer.len();
        self.buffer
            .extend(iter::repeat(GpuLightmap::default()).take(MAX_LIGHTMAPS - gpu_lightmap_count));
    }
}

impl RenderLightmapDescriptor {
    // Creates the texture array for the lightmaps associated with a mesh.
    fn create_lightmap_texture_array(&self, render_device: &RenderDevice) -> GpuImage {
        let (width, height) = (self.image_size.x as u32, self.image_size.y as u32);

        let texture = render_device.create_texture(&TextureDescriptor {
            label: Some("mesh_lightmap"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: self.array_indices.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.image_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: Some("mesh_lightmap"),
            format: Some(self.image_format),
            dimension: Some(TextureViewDimension::D2Array),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(self.array_indices.len() as u32),
        });

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("mesh_lightmap_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        GpuImage {
            texture,
            texture_view,
            texture_format: self.image_format,
            sampler,
            size: self.image_size,
            mip_level_count: 1,
        }
    }

    /// Copies all lightmaps for a mesh into an array texture.
    fn copy_lightmaps_to_texture(
        &self,
        command_encoder: &mut CommandEncoder,
        lightmap_image: &GpuImage,
        images: &RenderAssets<Image>,
    ) {
        let width = lightmap_image.size.x as u32;
        let height = lightmap_image.size.y as u32;

        // Copy each lightmap into each array slot.
        for (source_image_id, lightmap_array_index) in self.array_indices.iter() {
            let &LightmapTextureArrayIndex::Valid(lightmap_array_index) = lightmap_array_index
            else {
                continue;
            };
            let Some(source_image) = images.get(*source_image_id) else {
                continue;
            };

            command_encoder.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &source_image.texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &lightmap_image.texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: lightmap_array_index as u32,
                    },
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            )
        }
    }
}
