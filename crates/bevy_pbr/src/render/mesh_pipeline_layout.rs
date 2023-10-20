use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_render::{render_resource::BindGroupLayout, view::Msaa};

use crate::MeshPipelineKey;

#[derive(Clone)]
pub struct MeshPipelineViewLayout {
    pub bind_group_layout: BindGroupLayout,

    #[cfg(debug_assertions)]
    pub texture_count: usize,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct MeshPipelineViewLayoutKey: u32 {
        const MULTISAMPLED                = (1 << 0);
        const DEPTH_PREPASS               = (1 << 1);
        const NORMAL_PREPASS              = (1 << 2);
        const MOTION_VECTOR_PREPASS       = (1 << 3);
        const DEFERRED_PREPASS            = (1 << 4);
    }
}

impl MeshPipelineViewLayoutKey {
    pub const COUNT: usize = Self::all().bits() as usize + 1;

    /// Builds a unique label for each layout based on the flags
    pub fn label(&self) -> String {
        format!(
            "mesh_view_layout{}{}{}{}{}",
            if self.contains(MeshPipelineViewLayoutKey::MULTISAMPLED) {
                "_multisampled"
            } else {
                ""
            },
            if self.contains(MeshPipelineViewLayoutKey::DEPTH_PREPASS) {
                "_depth"
            } else {
                ""
            },
            if self.contains(MeshPipelineViewLayoutKey::NORMAL_PREPASS) {
                "_normal"
            } else {
                ""
            },
            if self.contains(MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS) {
                "_motion"
            } else {
                ""
            },
            if self.contains(MeshPipelineViewLayoutKey::DEFERRED_PREPASS) {
                "_deferred"
            } else {
                ""
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

        result
    }
}

impl From<Msaa> for MeshPipelineViewLayoutKey {
    fn from(value: Msaa) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if value.samples() > 1 {
            result |= MeshPipelineViewLayoutKey::MULTISAMPLED;
        }

        result
    }
}

impl From<Option<&ViewPrepassTextures>> for MeshPipelineViewLayoutKey {
    fn from(value: Option<&ViewPrepassTextures>) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if let Some(prepass_textures) = value {
            if prepass_textures.depth.is_some() {
                result |= MeshPipelineViewLayoutKey::DEPTH_PREPASS;
            }
            if prepass_textures.normal.is_some() {
                result |= MeshPipelineViewLayoutKey::NORMAL_PREPASS;
            }
            if prepass_textures.motion_vectors.is_some() {
                result |= MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS;
            }
            if prepass_textures.deferred.is_some() {
                result |= MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
            }
        }

        result
    }
}
