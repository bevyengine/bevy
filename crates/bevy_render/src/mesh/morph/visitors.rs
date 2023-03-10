use bevy_core::{Pod, Zeroable};
use bevy_math::Vec3;
use wgpu::VertexFormat;

use crate::mesh::{Mesh, MeshVertexAttribute};

/// The set of attributes conviniently accessible in the `gltf` crate.
///
/// Also the only ones useable with the current morph targets implementation.
/// See `morph.wgsl`.
pub const HARDCODED_ATTRIBUTES: &[MeshVertexAttribute] = &[
    Mesh::ATTRIBUTE_POSITION,
    Mesh::ATTRIBUTE_NORMAL,
    MeshVertexAttribute::new("Vertex_Tangent_Morph_Attribute", 7, VertexFormat::Float32x3),
];

/// Attributes **differences** used for morph targets.
///
/// See [`VisitMorphTargets`] for more informations.
#[derive(Copy, Clone, PartialEq, Pod, Zeroable, Default)]
#[repr(C)]
pub struct MorphAttributes {
    /// The vertex position difference between base mesh and this target.
    pub position: Vec3,
    /// The vertex normal difference between base mesh and this target.
    pub normal: Vec3,
    /// The vertex tangent difference between base mesh and this target.
    ///
    /// Note that tangents are a `Vec4`, but only the `xyz` components are
    /// animated, as the `w` component is the sign and cannot be animated.
    pub tangent: Vec3,
}
impl From<[Vec3; 3]> for MorphAttributes {
    fn from([position, normal, tangent]: [Vec3; 3]) -> Self {
        MorphAttributes {
            position,
            normal,
            tangent,
        }
    }
}
impl MorphAttributes {
    pub fn new(position: Vec3, normal: Vec3, tangent: Vec3) -> Self {
        MorphAttributes {
            position,
            normal,
            tangent,
        }
    }
}

/// All attributes of all vertices for a given [morph target][VisitMorphTargets].
pub trait VisitAttributes {
    /// Morph target attributes data for a single vertex.
    ///
    /// `Self` acts like an iterator, each call to `next_attributes` advances
    /// to the attributes for the next vertex.
    fn next_attributes(&mut self) -> Option<MorphAttributes>;
}
/// An accessor to read morph target attributes into bevy's mesh internal
/// morph target representation.
///
/// `Attributes` is an iterator where each individual item is all attributes
/// for all vertices used in a single target (think of "targets" as "poses"),
/// see [`VisitAttributes`].
///
/// Note that morph target attributes are **differences** between the base
/// mesh attribute value and the given pose's attribute value, following
/// closely the [glTF spec].
///
/// This is simplified pseudocode showing how bevy implements morph targets in
/// its vertex shader:
///
/// ```ignore
/// fn morph_vertex(mut vertex: Vertex) -> Vertex {
///     for (i, weight) in weights.enumerate() {
///         vertex.position += weight * morph(vertex.index, position_offset, i);
///         vertex.normal += weight * morph(vertex.index, normal_offset, i);
///         vertex.tangent += vec4(weight * morph(vertex.index, tangent_offset, i), 0.0);
///     }
///     return vertex;
/// }
/// ```
///
/// # Example
///
/// ```rust
/// use bevy_render::mesh::morph::{VisitAttributes, VisitMorphTargets, MorphAttributes};
/// use bevy_render::mesh::{Mesh, MeshVertexAttribute};
/// use bevy_math::Vec3;
/// use std::slice::Iter;
///
/// // Each entry in the slice `.0: &[T]` is a target.
/// // Consider `T: [&[Vec3]; 3]` a target. It has `3` attributes,
/// // each is a slice of `Vec3` (`&[ Vec3 ]`) where an entry is the
/// // attribute for the target for a single vertex.
/// struct TargetsCollection<'a>(&'a [[&'a [Vec3]; 3]]);
///
/// struct SingleTarget<'a>(Iter<'a, Vec3>, Iter<'a, Vec3>, Iter<'a, Vec3>);
///
/// impl<'a> VisitAttributes for SingleTarget<'a> {
///     fn next_attributes(&mut self) -> Option<MorphAttributes> {
///         let mut item = || {
///             Some(MorphAttributes::new(
///                 *self.0.next()?,
///                 *self.1.next()?,
///                 *self.2.next()?,
///             ))
///         };
///         Some(item().unwrap_or_else(MorphAttributes::default))
///     }
/// }
/// impl<'a> VisitMorphTargets for TargetsCollection<'a> {
///     type Visitor = SingleTarget<'a>;
///     type Attributes =
///         std::iter::Map<Iter<'a, [&'a [Vec3]; 3]>, fn(&[&'a [Vec3]; 3]) -> Self::Visitor>;
///     fn target_count(&self) -> usize {
///         self.0.len()
///     }
///     fn targets(&mut self) -> Self::Attributes {
///         self.0
///             .iter()
///             .map(|[p, n, t]| SingleTarget(p.iter(), n.iter(), t.iter()))
///     }
/// }
/// ```
///
/// ## Accounting
///
/// - a mesh has: `T` morph targets or poses
/// - a mesh has: `V` vertices
/// - a morph target has `A` animated attributes
/// - there is `V` animated attribute per mesh per morph target
///
/// [glTF spec]: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#morph-targets
pub trait VisitMorphTargets {
    type Visitor: VisitAttributes;
    type Attributes: Iterator<Item = Self::Visitor>;
    fn target_count(&self) -> usize;
    fn targets(&mut self) -> Self::Attributes;
}
