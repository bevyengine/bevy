use bevy_ecs::{
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_log::error;
use bevy_mesh::morph::{MorphAttributes, MorphBuildError, MAX_MORPH_WEIGHTS, MAX_TEXTURE_WIDTH};
use slotmap::SlotMap;
use wgpu::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};
use wgpu_types::TextureDataOrder;

use crate::{
    render_resource::{Buffer, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
};

/// An image formatted for use with [`bevy_mesh::morph::MorphWeights`] for
/// rendering the morph target, containing the vertex displacements.
///
/// We only use these if storage buffers aren't supported on the current
/// platform. Otherwise, we store the mesh displacements in a storage buffer,
/// managed by the mesh allocator.
#[derive(Clone, Debug)]
pub struct MorphTargetImage {
    /// The texture containing the vertex displacements.
    pub texture: Texture,
    /// A view into the texture, suitable for attaching to the vertex shader.
    pub texture_view: TextureView,
}

impl MorphTargetImage {
    /// Generate textures for each morph target.
    ///
    /// This accepts an "iterator of [`MorphAttributes`] iterators". Each item
    /// iterated in the top level iterator corresponds "the attributes of a
    /// specific morph target".
    ///
    /// Each pixel of the texture is a component of morph target animated
    /// attributes. So a set of 9 pixels is this morph's displacement for
    /// position, normal and tangents of a single vertex (each taking 3 pixels).
    pub fn new(
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        targets: &[MorphAttributes],
        vertex_count: usize,
    ) -> Result<Self, MorphBuildError> {
        let max = MAX_TEXTURE_WIDTH;
        let target_count = targets.len() / vertex_count;
        if target_count > MAX_MORPH_WEIGHTS {
            return Err(MorphBuildError::TooManyTargets { target_count });
        }
        let component_count = (vertex_count * MorphAttributes::COMPONENT_COUNT) as u32;
        let Some((Rect(width, height), padding)) = lowest_2d(component_count, max) else {
            return Err(MorphBuildError::TooManyAttributes {
                vertex_count,
                component_count,
            });
        };
        let data: Vec<u8> = targets
            .chunks(vertex_count)
            .flat_map(|attributes| {
                let layer_byte_count = (padding + component_count) as usize * size_of::<f32>();
                let mut buffer = Vec::with_capacity(layer_byte_count);
                for to_add in attributes {
                    buffer.extend_from_slice(bytemuck::bytes_of(&[
                        to_add.position,
                        to_add.normal,
                        to_add.tangent,
                    ]));
                }
                // Pad each layer so that they fit width * height
                buffer.extend(core::iter::repeat_n(0, padding as usize * size_of::<f32>()));
                debug_assert_eq!(buffer.len(), layer_byte_count);
                buffer
            })
            .collect();
        let extents = Extent3d {
            width,
            height,
            depth_or_array_layers: target_count as u32,
        };
        let texture = render_device.create_texture_with_data(
            render_queue,
            &TextureDescriptor {
                label: Some("morph target image"),
                size: extents,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::R32Float,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            &data,
        );
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: Some("morph target texture view"),
            ..TextureViewDescriptor::default()
        });
        Ok(MorphTargetImage {
            texture,
            texture_view,
        })
    }
}

slotmap::new_key_type! {
    pub struct MorphTargetImageKey;
}

#[derive(Debug)]
pub struct MorphTargetImageHandle(MorphTargetImageKey);

#[derive(Debug, Clone, Default)]
pub struct MorphTargetImages(SlotMap<MorphTargetImageKey, MorphTargetImage>);

impl MorphTargetImages {
    pub fn insert(&mut self, morph_target_image: MorphTargetImage) -> MorphTargetImageHandle {
        MorphTargetImageHandle(self.0.insert(morph_target_image))
    }

    pub fn remove(&mut self, handle: MorphTargetImageHandle) {
        self.0
            .remove(handle.0)
            .expect("MorphTargetImageHandle cannot double-free");
    }

    pub fn get(&self, handle: &MorphTargetImageHandle) -> &MorphTargetImage {
        self.0
            .get(handle.0)
            .expect("MorphTargetImageHandle cannot be use-after-free")
    }
}

/// Stores the images for all morph target displacement data, if the current
/// platform doesn't support storage buffers.
///
/// If the current platform does support storage buffers, the mesh allocator
/// stores displacement data instead.
#[derive(Debug, Resource)]
pub enum RenderMorphTargetAllocator {
    /// The variant used when the current platform doesn't support storage
    /// buffers.
    Image {
        morph_target_images: MorphTargetImages,
    },
    /// The variant used when the current platform does support storage buffers.
    ///
    /// In this case, this resource is empty, because the mesh allocator stores
    /// displacements instead.
    Storage,
}

impl FromWorld for RenderMorphTargetAllocator {
    fn from_world(world: &mut World) -> RenderMorphTargetAllocator {
        let render_device = world.resource::<RenderDevice>();
        if bevy_render::storage_buffers_are_unsupported(&render_device.limits()) {
            RenderMorphTargetAllocator::Image {
                morph_target_images: MorphTargetImages::default(),
            }
        } else {
            RenderMorphTargetAllocator::Storage
        }
    }
}

/// A reference to the resource in which morph displacements for a mesh are
/// stored.
#[derive(Clone, Copy)]
pub enum MorphTargetsResource<'a> {
    /// The [`MorphTargetImage`].
    ///
    /// This variant is used when storage buffers aren't supported on the
    /// current platform.
    Texture(&'a TextureView),

    /// The slab containing the morph target displacements.
    ///
    /// This variant is used when storage buffers are supported on the current
    /// platform.
    Storage(&'a Buffer),
}

impl RenderMorphTargetAllocator {
    /// Allocates morph target displacements for the given mesh.
    ///
    /// If storage buffers aren't supported on the current platform, this method
    /// creates a new [`MorphTargetImage`] and stores it inside the allocator.
    ///
    /// If storage buffers are supported on the current platform, this method
    /// does nothing, as morph target displacements are instead managed by the
    /// mesh allocator.
    pub fn allocate(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        targets: &[MorphAttributes],
        vertex_count: usize,
    ) -> Option<MorphTargetImageHandle> {
        match *self {
            RenderMorphTargetAllocator::Image {
                ref mut morph_target_images,
            } => match MorphTargetImage::new(render_device, render_queue, targets, vertex_count) {
                Ok(morph_target_image) => {
                    let handle = morph_target_images.insert(morph_target_image);
                    Some(handle)
                }
                Err(e) => {
                    error!("Failed to build morph target image for mesh {e:?}");
                    None
                }
            },
            RenderMorphTargetAllocator::Storage => {
                // Do nothing. Morph target displacements are managed by the
                // mesh allocator in this case.
                None
            }
        }
    }

    /// Frees the storage associated with morph target displacements for the
    /// mesh with the given ID.
    pub fn free(&mut self, handle: MorphTargetImageHandle) {
        match *self {
            RenderMorphTargetAllocator::Image {
                ref mut morph_target_images,
            } => morph_target_images.remove(handle),
            RenderMorphTargetAllocator::Storage => error!(
                "Attempted to free a morph target allocation {:?} when using storage allocator {:?}",
                handle,
                self
            ),
        }
    }

    /// Returns the [`MorphTargetImage`] containing the packed morph target
    /// displacements for the mesh with the given ID.
    pub fn get_image(&self, handle: &MorphTargetImageHandle) -> &MorphTargetImage {
        match *self {
            RenderMorphTargetAllocator::Image {
                ref morph_target_images,
            } => return morph_target_images.get(handle),
            RenderMorphTargetAllocator::Storage => panic!(
                "Attempted to get a morph target image with allocation {:?} when using storage allocator {:?}",
                handle,
                self
            ),
        }
    }
}

struct Rect(u32, u32);

/// Find the smallest rectangle of maximum edge size `max_edge` that contains
/// at least `min_includes` cells. `u32` is how many extra cells the rectangle
/// has.
///
/// The following rectangle contains 27 cells, and its longest edge is 9:
/// ```text
/// ----------------------------
/// |1 |2 |3 |4 |5 |6 |7 |8 |9 |
/// ----------------------------
/// |2 |  |  |  |  |  |  |  |  |
/// ----------------------------
/// |3 |  |  |  |  |  |  |  |  |
/// ----------------------------
/// ```
///
/// Returns `None` if `max_edge` is too small to build a rectangle
/// containing `min_includes` cells.
fn lowest_2d(min_includes: u32, max_edge: u32) -> Option<(Rect, u32)> {
    (1..=max_edge)
        .filter_map(|a| {
            let b = min_includes.div_ceil(a);
            let diff = (a * b).checked_sub(min_includes)?;
            Some((Rect(a, b), diff))
        })
        .filter_map(|(rect, diff)| (rect.1 <= max_edge).then_some((rect, diff)))
        .min_by_key(|(_, diff)| *diff)
}
