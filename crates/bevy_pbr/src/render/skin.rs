use core::mem::{self, size_of};
use std::sync::OnceLock;

use bevy_asset::{prelude::AssetChanged, Assets};
use bevy_camera::visibility::ViewVisibility;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_platform::collections::hash_map::Entry;
pub use bevy_render::mesh::skin::*;
use bevy_render::render_resource::BufferDescriptor;
use bevy_render::settings::WgpuLimits;
use bevy_render::sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet};
use bevy_render::{
    batching::NoAutomaticBatching,
    render_resource::BufferUsages,
    renderer::{RenderDevice, RenderQueue},
    Extract,
};
use bevy_transform::prelude::GlobalTransform;
use offset_allocator::Allocator;
use tracing::error;

pub fn skin_uniforms_from_world(world: &mut World) {
    let device = world.resource::<RenderDevice>();
    let buffer_usages = (if skins_use_uniform_buffers(&device.limits()) {
        BufferUsages::UNIFORM
    } else {
        BufferUsages::STORAGE
    }) | BufferUsages::COPY_DST;

    // Create the current and previous buffer with the minimum sizes.
    //
    // These will be swapped every frame.
    let current_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("skin uniform buffer"),
        size: MAX_JOINTS as u64 * size_of::<Mat4>() as u64,
        usage: buffer_usages,
        mapped_at_creation: false,
    });
    let prev_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("skin uniform buffer"),
        size: MAX_JOINTS as u64 * size_of::<Mat4>() as u64,
        usage: buffer_usages,
        mapped_at_creation: false,
    });

    let res = SkinUniforms {
        current_staging_buffer: vec![],
        current_buffer,
        prev_buffer,
        allocator: Allocator::new(MAX_TOTAL_JOINTS),
        skin_uniform_info: MainEntityHashMap::default(),
        joint_to_skins: MainEntityHashMap::default(),
        total_joints: 0,
    };

    world.insert_resource(res);
}

/// Returns true if skinning must use uniforms (and dynamic offsets) because
/// storage buffers aren't supported on the current platform.
pub fn skins_use_uniform_buffers(limits: &WgpuLimits) -> bool {
    static SKINS_USE_UNIFORM_BUFFERS: OnceLock<bool> = OnceLock::new();
    *SKINS_USE_UNIFORM_BUFFERS.get_or_init(|| limits.max_storage_buffers_per_shader_stage == 0)
}

/// Uploads the buffers containing the joints to the GPU.
pub fn prepare_skins(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    uniform: ResMut<SkinUniforms>,
) {
    let uniform = uniform.into_inner();

    if uniform.current_staging_buffer.is_empty() {
        return;
    }

    // Swap current and previous buffers.
    mem::swap(&mut uniform.current_buffer, &mut uniform.prev_buffer);

    // Resize the buffers if necessary. Include extra space equal to `MAX_JOINTS`
    // because we need to be able to bind a full uniform buffer's worth of data
    // if skins use uniform buffers on this platform.
    let needed_size = (uniform.current_staging_buffer.len() as u64 + MAX_JOINTS as u64)
        * size_of::<Mat4>() as u64;
    if uniform.current_buffer.size() < needed_size {
        let mut new_size = uniform.current_buffer.size();
        while new_size < needed_size {
            // 1.5× growth factor.
            new_size = (new_size + new_size / 2).next_multiple_of(4);
        }

        // Create the new buffers.
        let buffer_usages = if skins_use_uniform_buffers(&render_device.limits()) {
            BufferUsages::UNIFORM
        } else {
            BufferUsages::STORAGE
        } | BufferUsages::COPY_DST;
        uniform.current_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("skin uniform buffer"),
            usage: buffer_usages,
            size: new_size,
            mapped_at_creation: false,
        });
        uniform.prev_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("skin uniform buffer"),
            usage: buffer_usages,
            size: new_size,
            mapped_at_creation: false,
        });

        // We've created a new `prev_buffer` but we don't have the previous joint
        // data needed to fill it out correctly. Use the current joint data
        // instead.
        //
        // TODO: This is a bug - will cause motion blur to ignore joint movement
        // for one frame.
        render_queue.write_buffer(
            &uniform.prev_buffer,
            0,
            bytemuck::must_cast_slice(&uniform.current_staging_buffer[..]),
        );
    }

    // Write the data from `uniform.current_staging_buffer` into
    // `uniform.current_buffer`.
    render_queue.write_buffer(
        &uniform.current_buffer,
        0,
        bytemuck::must_cast_slice(&uniform.current_staging_buffer[..]),
    );

    // We don't need to write `uniform.prev_buffer` because we already wrote it
    // last frame, and the data should still be on the GPU.
}

// Notes on implementation:
// We define the uniform binding as an array<mat4x4<f32>, N> in the shader,
// where N is the maximum number of Mat4s we can fit in the uniform binding,
// which may be as little as 16kB or 64kB. But, we may not need all N.
// We may only need, for example, 10.
//
// If we used uniform buffers ‘normally’ then we would have to write a full
// binding of data for each dynamic offset binding, which is wasteful, makes
// the buffer much larger than it needs to be, and uses more memory bandwidth
// to transfer the data, which then costs frame time So @superdump came up
// with this design: just bind data at the specified offset and interpret
// the data at that offset as an array<T, N> regardless of what is there.
//
// So instead of writing N Mat4s when you only need 10, you write 10, and
// then pad up to the next dynamic offset alignment. Then write the next.
// And for the last dynamic offset binding, make sure there is a full binding
// of data after it so that the buffer is of size
// `last dynamic offset` + `array<mat4x4<f32>>`.
//
// Then when binding the first dynamic offset, the first 10 entries in the array
// are what you expect, but if you read the 11th you’re reading ‘invalid’ data
// which could be padding or could be from the next binding.
//
// In this way, we can pack ‘variable sized arrays’ into uniform buffer bindings
// which normally only support fixed size arrays. You just have to make sure
// in the shader that you only read the values that are valid for that binding.
pub fn extract_skins(
    skin_uniforms: ResMut<SkinUniforms>,
    skinned_meshes: Extract<Query<(Entity, &SkinnedMesh)>>,
    changed_skinned_meshes: Extract<
        Query<
            (Entity, &ViewVisibility, &SkinnedMesh),
            Or<(
                Changed<ViewVisibility>,
                Changed<SkinnedMesh>,
                AssetChanged<SkinnedMesh>,
            )>,
        >,
    >,
    skinned_mesh_inverse_bindposes: Extract<Res<Assets<SkinnedMeshInverseBindposes>>>,
    changed_transforms: Extract<Query<(Entity, &GlobalTransform), Changed<GlobalTransform>>>,
    joints: Extract<Query<&GlobalTransform>>,
    mut removed_skinned_meshes_query: Extract<RemovedComponents<SkinnedMesh>>,
) {
    let skin_uniforms = skin_uniforms.into_inner();

    // Find skins that have become visible or invisible on this frame. Allocate,
    // reallocate, or free space for them as necessary.
    add_or_delete_skins(
        skin_uniforms,
        &changed_skinned_meshes,
        &skinned_mesh_inverse_bindposes,
        &joints,
    );

    // Extract the transforms for all joints from the scene, and write them into
    // the staging buffer at the appropriate spot.
    extract_joints(
        skin_uniforms,
        &skinned_meshes,
        &changed_skinned_meshes,
        &skinned_mesh_inverse_bindposes,
        &changed_transforms,
        &joints,
    );

    // Delete skins that became invisible.
    for skinned_mesh_entity in removed_skinned_meshes_query.read() {
        // Only remove a skin if we didn't pick it up in `add_or_delete_skins`.
        // It's possible that a necessary component was removed and re-added in
        // the same frame.
        if !changed_skinned_meshes.contains(skinned_mesh_entity) {
            remove_skin(skin_uniforms, skinned_mesh_entity.into());
        }
    }
}

/// Searches for all skins that have become visible or invisible this frame and
/// allocations for them as necessary.
fn add_or_delete_skins(
    skin_uniforms: &mut SkinUniforms,
    changed_skinned_meshes: &Query<
        (Entity, &ViewVisibility, &SkinnedMesh),
        Or<(
            Changed<ViewVisibility>,
            Changed<SkinnedMesh>,
            AssetChanged<SkinnedMesh>,
        )>,
    >,
    skinned_mesh_inverse_bindposes: &Assets<SkinnedMeshInverseBindposes>,
    joints: &Query<&GlobalTransform>,
) {
    // Find every skinned mesh that changed one of (1) visibility; (2) joint
    // entities (part of `SkinnedMesh`); (3) the associated
    // `SkinnedMeshInverseBindposes` asset.
    for (skinned_mesh_entity, skinned_mesh_view_visibility, skinned_mesh) in changed_skinned_meshes
    {
        // Remove the skin if it existed last frame.
        let skinned_mesh_entity = MainEntity::from(skinned_mesh_entity);
        remove_skin(skin_uniforms, skinned_mesh_entity);

        // If the skin is invisible, we're done.
        if !(*skinned_mesh_view_visibility).get() {
            continue;
        }

        // Initialize the skin.
        add_skin(
            skinned_mesh_entity,
            skinned_mesh,
            skin_uniforms,
            skinned_mesh_inverse_bindposes,
            joints,
        );
    }
}

/// Extracts the global transforms of all joints and updates the staging buffer
/// as necessary.
fn extract_joints(
    skin_uniforms: &mut SkinUniforms,
    skinned_meshes: &Query<(Entity, &SkinnedMesh)>,
    changed_skinned_meshes: &Query<
        (Entity, &ViewVisibility, &SkinnedMesh),
        Or<(
            Changed<ViewVisibility>,
            Changed<SkinnedMesh>,
            AssetChanged<SkinnedMesh>,
        )>,
    >,
    skinned_mesh_inverse_bindposes: &Assets<SkinnedMeshInverseBindposes>,
    changed_transforms: &Query<(Entity, &GlobalTransform), Changed<GlobalTransform>>,
    joints: &Query<&GlobalTransform>,
) {
    // If the number of entities that changed transforms exceeds a certain
    // fraction (currently 25%) of the total joints in the scene, then skip
    // fine-grained change detection.
    //
    // Note that this is a crude heuristic, for performance reasons. It doesn't
    // consider the ratio of modified *joints* to total joints, only the ratio
    // of modified *entities* to total joints. Thus in the worst case we might
    // end up re-extracting all skins even though none of the joints changed.
    // But making the heuristic finer-grained would make it slower to evaluate,
    // and we don't want to lose performance.
    let threshold =
        (skin_uniforms.total_joints as f64 * JOINT_EXTRACTION_THRESHOLD_FACTOR).floor() as usize;

    if changed_transforms.iter().nth(threshold).is_some() {
        // Go ahead and re-extract all skins in the scene.
        for (skin_entity, skin) in skinned_meshes {
            extract_joints_for_skin(
                skin_entity.into(),
                skin,
                skin_uniforms,
                changed_skinned_meshes,
                skinned_mesh_inverse_bindposes,
                joints,
            );
        }
        return;
    }

    // Use fine-grained change detection to figure out only the skins that need
    // to have their joints re-extracted.
    let dirty_skins: MainEntityHashSet = changed_transforms
        .iter()
        .flat_map(|(joint, _)| skin_uniforms.joint_to_skins.get(&MainEntity::from(joint)))
        .flat_map(|skin_joint_mappings| skin_joint_mappings.iter())
        .copied()
        .collect();

    // Re-extract the joints for only those skins.
    for skin_entity in dirty_skins {
        let Ok((_, skin)) = skinned_meshes.get(*skin_entity) else {
            continue;
        };
        extract_joints_for_skin(
            skin_entity,
            skin,
            skin_uniforms,
            changed_skinned_meshes,
            skinned_mesh_inverse_bindposes,
            joints,
        );
    }
}

/// Extracts all joints for a single skin and writes their transforms into the
/// CPU staging buffer.
fn extract_joints_for_skin(
    skin_entity: MainEntity,
    skin: &SkinnedMesh,
    skin_uniforms: &mut SkinUniforms,
    changed_skinned_meshes: &Query<
        (Entity, &ViewVisibility, &SkinnedMesh),
        Or<(
            Changed<ViewVisibility>,
            Changed<SkinnedMesh>,
            AssetChanged<SkinnedMesh>,
        )>,
    >,
    skinned_mesh_inverse_bindposes: &Assets<SkinnedMeshInverseBindposes>,
    joints: &Query<&GlobalTransform>,
) {
    // If we initialized the skin this frame, we already populated all
    // the joints, so there's no need to populate them again.
    if changed_skinned_meshes.contains(*skin_entity) {
        return;
    }

    // Fetch information about the skin.
    let Some(skin_uniform_info) = skin_uniforms.skin_uniform_info.get(&skin_entity) else {
        return;
    };
    let Some(skinned_mesh_inverse_bindposes) =
        skinned_mesh_inverse_bindposes.get(&skin.inverse_bindposes)
    else {
        return;
    };

    // Calculate and write in the new joint matrices.
    for (joint_index, (&joint, skinned_mesh_inverse_bindpose)) in skin
        .joints
        .iter()
        .zip(skinned_mesh_inverse_bindposes.iter())
        .enumerate()
    {
        let Ok(joint_transform) = joints.get(joint) else {
            continue;
        };

        let joint_matrix = joint_transform.affine() * *skinned_mesh_inverse_bindpose;
        skin_uniforms.current_staging_buffer[skin_uniform_info.offset() as usize + joint_index] =
            joint_matrix;
    }
}

/// Allocates space for a new skin in the buffers, and populates its joints.
fn add_skin(
    skinned_mesh_entity: MainEntity,
    skinned_mesh: &SkinnedMesh,
    skin_uniforms: &mut SkinUniforms,
    skinned_mesh_inverse_bindposes: &Assets<SkinnedMeshInverseBindposes>,
    joints: &Query<&GlobalTransform>,
) {
    // Allocate space for the joints.
    let Some(allocation) = skin_uniforms.allocator.allocate(
        skinned_mesh
            .joints
            .len()
            .div_ceil(JOINTS_PER_ALLOCATION_UNIT as usize) as u32,
    ) else {
        error!(
            "Out of space for skin: {:?}. Tried to allocate space for {:?} joints.",
            skinned_mesh_entity,
            skinned_mesh.joints.len()
        );
        return;
    };

    // Store that allocation.
    let skin_uniform_info = SkinUniformInfo {
        allocation,
        joints: skinned_mesh
            .joints
            .iter()
            .map(|entity| MainEntity::from(*entity))
            .collect(),
    };

    let skinned_mesh_inverse_bindposes =
        skinned_mesh_inverse_bindposes.get(&skinned_mesh.inverse_bindposes);

    for (joint_index, &joint) in skinned_mesh.joints.iter().enumerate() {
        // Calculate the initial joint matrix.
        let skinned_mesh_inverse_bindpose =
            skinned_mesh_inverse_bindposes.and_then(|skinned_mesh_inverse_bindposes| {
                skinned_mesh_inverse_bindposes.get(joint_index)
            });
        let joint_matrix = match (skinned_mesh_inverse_bindpose, joints.get(joint)) {
            (Some(skinned_mesh_inverse_bindpose), Ok(transform)) => {
                transform.affine() * *skinned_mesh_inverse_bindpose
            }
            _ => Mat4::IDENTITY,
        };

        // Write in the new joint matrix, growing the staging buffer if
        // necessary.
        let buffer_index = skin_uniform_info.offset() as usize + joint_index;
        if skin_uniforms.current_staging_buffer.len() < buffer_index + 1 {
            skin_uniforms
                .current_staging_buffer
                .resize(buffer_index + 1, Mat4::IDENTITY);
        }
        skin_uniforms.current_staging_buffer[buffer_index] = joint_matrix;

        // Record the inverse mapping from the joint back to the skin. We use
        // this in order to perform fine-grained joint extraction.
        skin_uniforms
            .joint_to_skins
            .entry(MainEntity::from(joint))
            .or_default()
            .push(skinned_mesh_entity);
    }

    // Record the number of joints.
    skin_uniforms.total_joints += skinned_mesh.joints.len();

    skin_uniforms
        .skin_uniform_info
        .insert(skinned_mesh_entity, skin_uniform_info);
}

/// Deallocates a skin and removes it from the [`SkinUniforms`].
fn remove_skin(skin_uniforms: &mut SkinUniforms, skinned_mesh_entity: MainEntity) {
    let Some(old_skin_uniform_info) = skin_uniforms.skin_uniform_info.remove(&skinned_mesh_entity)
    else {
        return;
    };

    // Free the allocation.
    skin_uniforms
        .allocator
        .free(old_skin_uniform_info.allocation);

    // Remove the inverse mapping from each joint back to the skin.
    for &joint in &old_skin_uniform_info.joints {
        if let Entry::Occupied(mut entry) = skin_uniforms.joint_to_skins.entry(joint) {
            entry.get_mut().retain(|skin| *skin != skinned_mesh_entity);
            if entry.get_mut().is_empty() {
                entry.remove();
            }
        }
    }

    // Update the total number of joints.
    skin_uniforms.total_joints -= old_skin_uniform_info.joints.len();
}

// NOTE: The skinned joints uniform buffer has to be bound at a dynamic offset per
// entity and so cannot currently be batched on WebGL 2.
pub fn no_automatic_skin_batching(
    mut commands: Commands,
    query: Query<Entity, (With<SkinnedMesh>, Without<NoAutomaticBatching>)>,
    render_device: Res<RenderDevice>,
) {
    if !skins_use_uniform_buffers(&render_device.limits()) {
        return;
    }

    for entity in &query {
        commands.entity(entity).try_insert(NoAutomaticBatching);
    }
}
