use core::{iter, mem};

use bevy_camera::visibility::ViewVisibility;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_mesh::morph::{MeshMorphWeights, MorphWeights, MAX_MORPH_WEIGHTS};
use bevy_platform::collections::hash_map::Entry;
use bevy_render::mesh::allocator::MeshAllocator;
use bevy_render::mesh::RenderMesh;
use bevy_render::render_asset::RenderAssets;
use bevy_render::render_resource::ShaderType;
use bevy_render::sync_world::{MainEntity, MainEntityHashMap};
use bevy_render::{
    batching::NoAutomaticBatching,
    render_resource::{BufferUsages, RawBufferVec},
    renderer::{RenderDevice, RenderQueue},
    Extract,
};
use bytemuck::{NoUninit, Pod, Zeroable};

use crate::{skin, RenderMeshInstances};

#[derive(Component)]
pub struct MorphIndex {
    pub index: u32,
}

/// The index of the [`GpuMorphDescriptor`] in the `morph_descriptors` buffer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Deref, DerefMut)]
pub struct MorphDescriptorIndex(pub u32);

/// Maps each mesh affected by morph targets to the applicable offset within the
/// [`MorphUniforms`] buffer.
///
/// We store both the current frame's mapping and the previous frame's mapping
/// for the purposes of motion vector calculation.
#[derive(Resource)]
pub enum MorphIndices {
    /// The variant used when storage buffers aren't supported on the current
    /// platform.
    Uniform {
        /// Maps each entity with a morphed mesh to the appropriate offset within
        /// [`MorphUniforms::current_buffer`].
        current: MainEntityHashMap<MorphIndex>,

        /// Maps each entity with a morphed mesh to the appropriate offset within
        /// [`MorphUniforms::prev_buffer`].
        prev: MainEntityHashMap<MorphIndex>,
    },

    /// The variant used when storage buffers are supported on the current
    /// platform.
    Storage {
        /// Maps each entity with a morphed mesh to the [`MorphWeightsInfo`].
        morph_weights_info: MainEntityHashMap<MorphWeightsInfo>,
        /// Maps each entity with a morphed mesh to the [`GpuMorphDescriptor`]
        /// in the `morph_descriptors` buffer.
        gpu_descriptor_indices: MainEntityHashMap<MorphDescriptorIndex>,
        /// Indices in the `morph_descriptors` buffer available for use.
        gpu_descriptor_free_list: Vec<MorphDescriptorIndex>,
    },
}

/// Information that the CPU needs about each morh target for the purposes of
/// weight calculation.
#[derive(Clone, Copy)]
pub struct MorphWeightsInfo {
    /// The offset to the first weight for this mesh instance in the
    /// `morph_weights` buffer.
    current_weight_offset: u32,
    /// The offset to the first weight for this mesh instance in the
    /// `prev_morph_weights` buffer, if applicable
    pub(crate) prev_weight_offset: Option<u32>,
    /// The total number of morph targets that this mesh instance has.
    weight_count: u32,
}

impl FromWorld for MorphIndices {
    fn from_world(world: &mut World) -> MorphIndices {
        let render_device = world.resource::<RenderDevice>();

        if skin::skins_use_uniform_buffers(&render_device.limits()) {
            MorphIndices::Uniform {
                current: MainEntityHashMap::default(),
                prev: MainEntityHashMap::default(),
            }
        } else {
            MorphIndices::Storage {
                morph_weights_info: MainEntityHashMap::default(),
                gpu_descriptor_indices: MainEntityHashMap::default(),
                gpu_descriptor_free_list: vec![],
            }
        }
    }
}

/// The GPU buffers containing morph weights for all meshes with morph targets.
///
/// This is double-buffered: we store the weights of the previous frame in
/// addition to those of the current frame. This is for motion vector
/// calculation. Every frame, we swap buffers and reuse the morph target weight
/// buffer from two frames ago for the current frame.
#[derive(Resource)]
pub struct MorphUniforms {
    /// The morph weights for the current frame.
    pub current_buffer: RawBufferVec<f32>,
    /// The morph weights for the previous frame.
    pub prev_buffer: RawBufferVec<f32>,
    /// Information that the GPU needs about each morph target.
    ///
    /// This is only present if morph targets use storage buffers. If the
    /// platform doesn't support storage buffers, we're using morph target
    /// images instead, and the shader can determine the relevant info from the
    /// texture dimensions.
    pub descriptors_buffer: Option<RawBufferVec<GpuMorphDescriptor>>,
}

impl FromWorld for MorphUniforms {
    fn from_world(world: &mut World) -> MorphUniforms {
        let render_device = world.resource::<RenderDevice>();

        let skins_use_uniform_buffers = skin::skins_use_uniform_buffers(&render_device.limits());

        let buffer_usages = BufferUsages::COPY_DST
            | (if skins_use_uniform_buffers {
                BufferUsages::UNIFORM
            } else {
                BufferUsages::STORAGE
            });

        MorphUniforms {
            current_buffer: RawBufferVec::new(buffer_usages),
            prev_buffer: RawBufferVec::new(buffer_usages),
            descriptors_buffer: if skins_use_uniform_buffers {
                None
            } else {
                Some(RawBufferVec::new(
                    BufferUsages::COPY_DST | BufferUsages::STORAGE,
                ))
            },
        }
    }
}

impl MorphUniforms {
    /// Swaps the current buffer and previous buffer, and clears out the new
    /// current buffer in preparation for a new frame.
    fn prepare_for_new_frame(&mut self) {
        mem::swap(&mut self.current_buffer, &mut self.prev_buffer);
        self.current_buffer.clear();
    }
}

impl MorphIndices {
    /// Returns the index of the morph descriptor in the morph descriptor table
    /// for the given entity.
    ///
    /// As morph descriptors are only present if the platform supports storage
    /// buffers, this method returns `None` if the platform doesn't support
    /// them.
    pub(crate) fn morph_descriptor_index(
        &self,
        main_entity: MainEntity,
    ) -> Option<MorphDescriptorIndex> {
        match *self {
            MorphIndices::Uniform { .. } => None,
            MorphIndices::Storage {
                ref gpu_descriptor_indices,
                ..
            } => gpu_descriptor_indices.get(&main_entity).copied(),
        }
    }
}

/// Information that the GPU needs about a single mesh instance that uses morph
/// targets.
#[derive(Clone, Copy, Default, ShaderType, Pod, Zeroable)]
#[repr(C)]
pub struct GpuMorphDescriptor {
    /// The index of the first morph target weight in the `morph_weights` array.
    pub current_weights_offset: u32,
    /// The index of the first morph target weight in the `prev_morph_weights`
    /// array.
    pub prev_weights_offset: u32,
    /// The index of the first morph target for this mesh in the
    /// `MorphAttributes` array.
    pub targets_offset: u32,
    /// The number of vertices in the mesh.
    pub vertex_count: u32,
    /// The number of morph targets this mesh has.
    pub weight_count: u32,
}

/// A system that writes the buffers inside [`MorphUniforms`] to the GPU.
pub fn write_morph_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut uniform: ResMut<MorphUniforms>,
) {
    if uniform.current_buffer.is_empty() {
        return;
    }
    let len = uniform.current_buffer.len();
    uniform.current_buffer.reserve(len, &render_device);
    uniform
        .current_buffer
        .write_buffer(&render_device, &render_queue);

    // We don't need to write `uniform.prev_buffer` because we already wrote it
    // last frame, and the data should still be on the GPU.

    if let Some(ref mut descriptors_buffer) = uniform.descriptors_buffer {
        if descriptors_buffer.is_empty() {
            descriptors_buffer.push(GpuMorphDescriptor::default());
        }
        descriptors_buffer.write_buffer(&render_device, &render_queue);
    }
}

const fn can_align(step: usize, target: usize) -> bool {
    step.is_multiple_of(target) || target.is_multiple_of(step)
}

const WGPU_MIN_ALIGN: usize = 256;

/// Align a [`RawBufferVec`] to `N` bytes by padding the end with `T::default()` values.
fn add_to_alignment<T: NoUninit + Default>(buffer: &mut RawBufferVec<T>) {
    let n = WGPU_MIN_ALIGN;
    let t_size = size_of::<T>();
    if !can_align(n, t_size) {
        // This panic is stripped at compile time, due to n, t_size and can_align being const
        panic!(
            "RawBufferVec should contain only types with a size multiple or divisible by {n}, \
            {} has a size of {t_size}, which is neither multiple or divisible by {n}",
            core::any::type_name::<T>()
        );
    }

    let buffer_size = buffer.len();
    let byte_size = t_size * buffer_size;
    let bytes_over_n = byte_size % n;
    if bytes_over_n == 0 {
        return;
    }
    let bytes_to_add = n - bytes_over_n;
    let ts_to_add = bytes_to_add / t_size;
    buffer.extend(iter::repeat_with(T::default).take(ts_to_add));
}

// Notes on implementation: see comment on top of the extract_skins system in skin module.
// This works similarly, but for `f32` instead of `Mat4`
pub fn extract_morphs(
    morph_indices: ResMut<MorphIndices>,
    uniform: ResMut<MorphUniforms>,
    query: Extract<Query<(Entity, &ViewVisibility, &MeshMorphWeights)>>,
    weights_query: Extract<Query<&MorphWeights>>,
    render_device: Res<RenderDevice>,
) {
    // Borrow check workaround.
    let (morph_indices, uniform) = (morph_indices.into_inner(), uniform.into_inner());

    let morphs_use_uniform_buffers = skin::skins_use_uniform_buffers(&render_device.limits());

    // Swap buffers. We need to keep the previous frame's buffer around for the
    // purposes of motion vector computation.
    let maybe_old_morph_target_info = match *morph_indices {
        MorphIndices::Uniform {
            ref mut current,
            ref mut prev,
        } => {
            mem::swap(current, prev);
            current.clear();
            None
        }
        MorphIndices::Storage {
            morph_weights_info: ref mut morph_target_info,
            ..
        } => Some(mem::take(morph_target_info)),
    };

    uniform.prepare_for_new_frame();

    // Loop over each entity with morph targets.
    for (entity, view_visibility, mesh_weights) in &query {
        if !view_visibility.get() {
            continue;
        }
        let Ok(weights) = (match mesh_weights {
            MeshMorphWeights::Reference(entity) => {
                weights_query.get(*entity).map(MorphWeights::weights)
            }
            MeshMorphWeights::Value { weights } => Ok(weights.as_slice()),
        }) else {
            continue;
        };

        // Write the weights to the buffer. If we're using uniform buffers, then
        // we have to pad out the buffer to its fixed length.
        let start = uniform.current_buffer.len();
        if morphs_use_uniform_buffers {
            let legal_weights = weights
                .iter()
                .chain(iter::repeat(&0.0))
                .take(MAX_MORPH_WEIGHTS)
                .copied();
            uniform.current_buffer.extend(legal_weights);
            add_to_alignment::<f32>(&mut uniform.current_buffer);
        } else {
            uniform.current_buffer.extend(weights.iter().copied());
        }

        // Find the index of the weights for the previous frame in the buffer.
        let maybe_prev_weights_offset =
            maybe_old_morph_target_info
                .as_ref()
                .and_then(|old_morph_target_info| {
                    old_morph_target_info
                        .get(&MainEntity::from(entity))
                        .map(|morph_target_info| morph_target_info.current_weight_offset)
                });

        // Store the location of the weights for future use.
        match *morph_indices {
            MorphIndices::Uniform {
                ref mut current, ..
            } => {
                let index = (start * size_of::<f32>()) as u32;
                current.insert(entity.into(), MorphIndex { index });
            }
            MorphIndices::Storage {
                morph_weights_info: ref mut morph_target_info,
                ..
            } => {
                morph_target_info.insert(
                    entity.into(),
                    MorphWeightsInfo {
                        current_weight_offset: start as u32,
                        prev_weight_offset: maybe_prev_weights_offset,
                        weight_count: weights.len() as u32,
                    },
                );
            }
        }
    }
}

/// A system that writes [`GpuMorphDescriptor`] values to the [`MorphUniforms`]
/// for each mesh instance with morph targets.
///
/// As morph descriptors are only used when the platform supports storage
/// buffers, if the platform doesn't support storage buffers, this system does
/// nothing.
pub fn prepare_morph_descriptors(
    mut morph_indices: ResMut<MorphIndices>,
    mut morph_uniforms: ResMut<MorphUniforms>,
    render_mesh_instances: Res<RenderMeshInstances>,
    meshes: Res<RenderAssets<RenderMesh>>,
    mesh_allocator: Res<MeshAllocator>,
) {
    // Don't do anything unless the platform supports storage buffers.
    let (
        &mut MorphIndices::Storage {
            morph_weights_info: ref morph_target_info,
            ref mut gpu_descriptor_indices,
            ref mut gpu_descriptor_free_list,
        },
        &mut Some(ref mut descriptors_buffer),
    ) = (&mut *morph_indices, &mut morph_uniforms.descriptors_buffer)
    else {
        return;
    };

    for (&morph_target_main_entity, morph_target_info) in morph_target_info {
        let Some(mesh_id) = render_mesh_instances.mesh_asset_id(morph_target_main_entity) else {
            continue;
        };
        let Some(mesh) = meshes.get(mesh_id) else {
            continue;
        };
        let Some(morph_targets_slice) = mesh_allocator.mesh_morph_target_slice(&mesh_id) else {
            continue;
        };

        // Create our morph descriptor.
        let morph_descriptor = GpuMorphDescriptor {
            current_weights_offset: morph_target_info.current_weight_offset,
            prev_weights_offset: morph_target_info.prev_weight_offset.unwrap_or(!0),
            targets_offset: morph_targets_slice.range.start,
            vertex_count: mesh.vertex_count,
            weight_count: morph_target_info.weight_count,
        };

        // Place it in the descriptors buffer. Note that if the morph target
        // descriptor for an entity was in the buffer last frame, then it must
        // be at the same index this frame. That's because the
        // `MeshInputUniform` stores the index of the morph target descriptor,
        // and `MeshInputUniform`s aren't updated unless the mesh instance
        // changes.
        let descriptor_index;
        match gpu_descriptor_indices.entry(morph_target_main_entity) {
            Entry::Occupied(occupied_entry) => {
                descriptor_index = *occupied_entry.get();
                descriptors_buffer.set(descriptor_index.0, morph_descriptor);
            }
            Entry::Vacant(vacant_entry) => {
                match gpu_descriptor_free_list.pop() {
                    Some(free_descriptor_index) => {
                        descriptor_index = free_descriptor_index;
                        descriptors_buffer.set(descriptor_index.0, morph_descriptor);
                    }
                    None => {
                        descriptor_index =
                            MorphDescriptorIndex(descriptors_buffer.push(morph_descriptor) as u32);
                    }
                }
                vacant_entry.insert(descriptor_index);
            }
        };

        // Note where we wrote it.
        gpu_descriptor_indices.insert(morph_target_main_entity, descriptor_index);
    }

    // Expire descriptor indices corresponding to entities no longer present.
    gpu_descriptor_indices.retain(|morph_target_main_entity, descriptor_index| {
        let live = morph_target_info.contains_key(morph_target_main_entity);
        if !live {
            gpu_descriptor_free_list.push(*descriptor_index);
        }
        live
    });
}

// NOTE: Because morph targets require per-morph target texture bindings, they cannot
// currently be batched on platforms without storage buffers.
pub fn no_automatic_morph_batching(
    mut commands: Commands,
    query: Query<Entity, (With<MeshMorphWeights>, Without<NoAutomaticBatching>)>,
    render_device: Res<RenderDevice>,
) {
    // We *can* batch mesh instances with morph targets if the platform supports
    // storage buffers.
    if !skin::skins_use_uniform_buffers(&render_device.limits()) {
        return;
    }

    for entity in &query {
        commands.entity(entity).try_insert(NoAutomaticBatching);
    }
}
