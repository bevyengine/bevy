#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

extern crate alloc;
extern crate core;

mod components;
mod conversions;
mod index;
mod mesh;
#[cfg(feature = "bevy_mikktspace")]
mod mikktspace;
#[cfg(feature = "morph")]
pub mod morph;
pub mod primitives;
pub mod skinning;
mod vertex;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{AssetApp, AssetEventSystems};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bitflags::bitflags;
pub use components::*;
pub use index::*;
pub use mesh::*;
#[cfg(feature = "bevy_mikktspace")]
pub use mikktspace::*;
pub use primitives::*;
pub use vertex::*;
pub use wgpu_types::VertexFormat;

/// The mesh prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "morph")]
    pub use crate::morph::MorphWeights;
    #[doc(hidden)]
    pub use crate::{primitives::MeshBuilder, primitives::Meshable, Mesh, Mesh2d, Mesh3d};
}

bitflags! {
    /// Our base mesh pipeline key bits start from the highest bit and go
    /// downward. The PBR mesh pipeline key bits start from the lowest bit and
    /// go upward. This allows the PBR bits in the downstream crate `bevy_pbr`
    /// to coexist in the same field without any shifts.
    #[derive(Clone, Debug)]
    pub struct BaseMeshPipelineKey: u64 {
        const MORPH_TARGETS = 1 << Self::MORPH_TARGETS_SHIFT_BITS;

        const PRIMITIVE_TOPOLOGY_RESERVED_BITS  = Self::PRIMITIVE_TOPOLOGY_MASK_BITS << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;

        const STRIP_INDEX_FORMAT_RESERVED_BITS = Self::STRIP_INDEX_FORMAT_MASK_BITS << Self::STRIP_INDEX_FORMAT_SHIFT_BITS;
        const STRIP_INDEX_FORMAT_NONE = 0 << Self::STRIP_INDEX_FORMAT_SHIFT_BITS;
        const STRIP_INDEX_FORMAT_U32  = 1 << Self::STRIP_INDEX_FORMAT_SHIFT_BITS;
        const STRIP_INDEX_FORMAT_U16  = 2 << Self::STRIP_INDEX_FORMAT_SHIFT_BITS;
    }
}

/// Adds [`Mesh`] as an asset.
#[derive(Default)]
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>()
            .init_asset::<skinning::SkinnedMeshInverseBindposes>()
            .register_asset_reflect::<Mesh>()
            .add_systems(
                PostUpdate,
                mark_3d_meshes_as_changed_if_their_assets_changed.after(AssetEventSystems),
            );
    }
}

impl BaseMeshPipelineKey {
    pub const MORPH_TARGETS_SHIFT_BITS: u64 = (u64::BITS - 1) as u64;

    pub const PRIMITIVE_TOPOLOGY_MASK_BITS: u64 = 0b111;
    pub const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u64 =
        Self::MORPH_TARGETS_SHIFT_BITS - Self::PRIMITIVE_TOPOLOGY_MASK_BITS.count_ones() as u64;

    pub const STRIP_INDEX_FORMAT_MASK_BITS: u64 = 0b11;
    pub const STRIP_INDEX_FORMAT_SHIFT_BITS: u64 = Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS
        - Self::STRIP_INDEX_FORMAT_MASK_BITS.count_ones() as u64;

    /// Create a [`BaseMeshPipelineKey`] from mesh primitive topology and index format.
    ///
    /// For non-strip topologies, [`Self::STRIP_INDEX_FORMAT_NONE`] is set regardless of the `strip_index_format` argument.
    pub fn from_primitive_topology_and_strip_index(
        primitive_topology: PrimitiveTopology,
        strip_index_format: Option<wgpu_types::IndexFormat>,
    ) -> Self {
        let index_bits = if primitive_topology.is_strip() {
            match strip_index_format {
                None => BaseMeshPipelineKey::STRIP_INDEX_FORMAT_NONE,
                Some(index_format) => match index_format {
                    wgpu_types::IndexFormat::Uint16 => BaseMeshPipelineKey::STRIP_INDEX_FORMAT_U16,
                    wgpu_types::IndexFormat::Uint32 => BaseMeshPipelineKey::STRIP_INDEX_FORMAT_U32,
                },
            }
        } else {
            BaseMeshPipelineKey::STRIP_INDEX_FORMAT_NONE
        }
        .bits();
        let primitive_topology_bits = ((primitive_topology as u64)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits | index_bits)
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits = (self.bits() >> Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u64 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u64 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u64 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u64 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u64 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
        }
    }
}
