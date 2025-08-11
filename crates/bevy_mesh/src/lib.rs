#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

extern crate alloc;
extern crate core;

mod components;
mod conversions;
mod index;
mod mesh;
mod mikktspace;
pub mod morph;
pub mod primitives;
pub mod skinning;
mod vertex;
use bevy_ecs::schedule::SystemSet;
use bitflags::bitflags;
pub use components::*;
pub use index::*;
pub use mesh::*;
pub use mikktspace::*;
pub use primitives::*;
pub use vertex::*;
pub use wgpu_types::VertexFormat;

/// The mesh prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        morph::MorphWeights, primitives::MeshBuilder, primitives::Meshable, Mesh, Mesh2d, Mesh3d,
    };
}

bitflags! {
    /// Our base mesh pipeline key bits start from the highest bit and go
    /// downward. The PBR mesh pipeline key bits start from the lowest bit and
    /// go upward. This allows the PBR bits in the downstream crate `bevy_pbr`
    /// to coexist in the same field without any shifts.
    #[derive(Clone, Debug)]
    pub struct BaseMeshPipelineKey: u64 {
        const MORPH_TARGETS = 1 << (u64::BITS - 1);
    }
}

impl BaseMeshPipelineKey {
    pub const PRIMITIVE_TOPOLOGY_MASK_BITS: u64 = 0b111;
    pub const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u64 =
        (u64::BITS - 1 - Self::PRIMITIVE_TOPOLOGY_MASK_BITS.count_ones()) as u64;

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u64)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits_retain(primitive_topology_bits)
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

/// `bevy_render::mesh::inherit_weights` runs in this `SystemSet`
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct InheritWeights;
