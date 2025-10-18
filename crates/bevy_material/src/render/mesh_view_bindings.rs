use crate::{render::MeshPipelineKey, render_resource::*};
use alloc::sync::Arc;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::resource::Resource;

#[cfg(debug_assertions)]
use {crate::render::MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES, bevy_utils::once, tracing::warn};

#[derive(Clone)]
pub struct MeshPipelineViewLayout {
    pub main_layout: BindGroupLayoutDescriptor,
    pub binding_array_layout: BindGroupLayoutDescriptor,
    pub empty_layout: BindGroupLayoutDescriptor,

    #[cfg(debug_assertions)]
    pub texture_count: usize,
}

bitflags::bitflags! {
    /// A key that uniquely identifies a [`MeshPipelineViewLayout`].
    ///
    /// Used to generate all possible layouts for the mesh pipeline in [`generate_view_layouts`],
    /// so special care must be taken to not add too many flags, as the number of possible layouts
    /// will grow exponentially.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct MeshPipelineViewLayoutKey: u32 {
        const MULTISAMPLED                = 1 << 0;
        const DEPTH_PREPASS               = 1 << 1;
        const NORMAL_PREPASS              = 1 << 2;
        const MOTION_VECTOR_PREPASS       = 1 << 3;
        const DEFERRED_PREPASS            = 1 << 4;
        const OIT_ENABLED                 = 1 << 5;
    }
}

impl MeshPipelineViewLayoutKey {
    // The number of possible layouts
    pub const COUNT: usize = Self::all().bits() as usize + 1;

    /// Builds a unique label for each layout based on the flags
    pub fn label(&self) -> String {
        use MeshPipelineViewLayoutKey as Key;

        format!(
            "mesh_view_layout{}{}{}{}{}{}",
            if self.contains(Key::MULTISAMPLED) {
                "_multisampled"
            } else {
                Default::default()
            },
            if self.contains(Key::DEPTH_PREPASS) {
                "_depth"
            } else {
                Default::default()
            },
            if self.contains(Key::NORMAL_PREPASS) {
                "_normal"
            } else {
                Default::default()
            },
            if self.contains(Key::MOTION_VECTOR_PREPASS) {
                "_motion"
            } else {
                Default::default()
            },
            if self.contains(Key::DEFERRED_PREPASS) {
                "_deferred"
            } else {
                Default::default()
            },
            if self.contains(Key::OIT_ENABLED) {
                "_oit"
            } else {
                Default::default()
            },
        )
    }
}

impl From<MeshPipelineKey> for MeshPipelineViewLayoutKey {
    fn from(value: MeshPipelineKey) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if value.msaa_samples() > 1 {
            result |= MeshPipelineViewLayoutKey::MULTISAMPLED;
        }
        if value.contains(MeshPipelineKey::DEPTH_PREPASS) {
            result |= MeshPipelineViewLayoutKey::DEPTH_PREPASS;
        }
        if value.contains(MeshPipelineKey::NORMAL_PREPASS) {
            result |= MeshPipelineViewLayoutKey::NORMAL_PREPASS;
        }
        if value.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            result |= MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS;
        }
        if value.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            result |= MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
        }
        if value.contains(MeshPipelineKey::OIT_ENABLED) {
            result |= MeshPipelineViewLayoutKey::OIT_ENABLED;
        }

        result
    }
}

/// Stores the view layouts for every combination of pipeline keys.
///
/// This is wrapped in an [`Arc`] so that it can be efficiently cloned and
/// placed inside specializable pipeline types.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct MeshPipelineViewLayouts(
    pub Arc<[MeshPipelineViewLayout; MeshPipelineViewLayoutKey::COUNT]>,
);

impl MeshPipelineViewLayouts {
    pub fn get_view_layout(
        &self,
        layout_key: MeshPipelineViewLayoutKey,
    ) -> &MeshPipelineViewLayout {
        let index = layout_key.bits() as usize;
        let layout = &self[index];

        #[cfg(debug_assertions)]
        if layout.texture_count > MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES {
            // Issue our own warning here because Naga's error message is a bit cryptic in this situation
            once!(warn!("Too many textures in mesh pipeline view layout, this might cause us to hit `wgpu::Limits::max_sampled_textures_per_shader_stage` in some environments."));
        }

        layout
    }
}
