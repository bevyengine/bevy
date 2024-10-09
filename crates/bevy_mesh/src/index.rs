use bevy_reflect::Reflect;
use core::iter::FusedIterator;
use derive_more::derive::{Display, Error};
use wgpu::IndexFormat;

/// A disjunction of four iterators. This is necessary to have a well-formed type for the output
/// of [`Mesh::triangles`](super::Mesh::triangles), which produces iterators of four different types depending on the
/// branch taken.
pub(crate) enum FourIterators<A, B, C, D> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
}

impl<A, B, C, D, I> Iterator for FourIterators<A, B, C, D>
where
    A: Iterator<Item = I>,
    B: Iterator<Item = I>,
    C: Iterator<Item = I>,
    D: Iterator<Item = I>,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FourIterators::First(iter) => iter.next(),
            FourIterators::Second(iter) => iter.next(),
            FourIterators::Third(iter) => iter.next(),
            FourIterators::Fourth(iter) => iter.next(),
        }
    }
}

/// An error that occurred while trying to invert the winding of a [`Mesh`](super::Mesh).
#[derive(Debug, Error, Display)]
pub enum MeshWindingInvertError {
    /// This error occurs when you try to invert the winding for a mesh with [`PrimitiveTopology::PointList`](super::PrimitiveTopology::PointList).
    #[display("Mesh winding invertation does not work for primitive topology `PointList`")]
    WrongTopology,

    /// This error occurs when you try to invert the winding for a mesh with
    /// * [`PrimitiveTopology::TriangleList`](super::PrimitiveTopology::TriangleList), but the indices are not in chunks of 3.
    /// * [`PrimitiveTopology::LineList`](super::PrimitiveTopology::LineList), but the indices are not in chunks of 2.
    #[display("Indices weren't in chunks according to topology")]
    AbruptIndicesEnd,
}

/// An error that occurred while trying to extract a collection of triangles from a [`Mesh`](super::Mesh).
#[derive(Debug, Error, Display)]
pub enum MeshTrianglesError {
    #[display("Source mesh does not have primitive topology TriangleList or TriangleStrip")]
    WrongTopology,

    #[display("Source mesh lacks position data")]
    MissingPositions,

    #[display("Source mesh position data is not Float32x3")]
    PositionsFormat,

    #[display("Source mesh lacks face index data")]
    MissingIndices,

    #[display("Face index data references vertices that do not exist")]
    BadIndices,
}

/// An array of indices into the [`VertexAttributeValues`](super::VertexAttributeValues) for a mesh.
///
/// It describes the order in which the vertex attributes should be joined into faces.
#[derive(Debug, Clone, Reflect)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Indices {
    /// Returns an iterator over the indices.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        match self {
            Indices::U16(vec) => IndicesIter::U16(vec.iter()),
            Indices::U32(vec) => IndicesIter::U32(vec.iter()),
        }
    }

    /// Returns the number of indices.
    pub fn len(&self) -> usize {
        match self {
            Indices::U16(vec) => vec.len(),
            Indices::U32(vec) => vec.len(),
        }
    }

    /// Returns `true` if there are no indices.
    pub fn is_empty(&self) -> bool {
        match self {
            Indices::U16(vec) => vec.is_empty(),
            Indices::U32(vec) => vec.is_empty(),
        }
    }
}

/// An Iterator for the [`Indices`].
enum IndicesIter<'a> {
    U16(core::slice::Iter<'a, u16>),
    U32(core::slice::Iter<'a, u32>),
}

impl Iterator for IndicesIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IndicesIter::U16(iter) => iter.next().map(|val| *val as usize),
            IndicesIter::U32(iter) => iter.next().map(|val| *val as usize),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            IndicesIter::U16(iter) => iter.size_hint(),
            IndicesIter::U32(iter) => iter.size_hint(),
        }
    }
}

impl<'a> ExactSizeIterator for IndicesIter<'a> {}
impl<'a> FusedIterator for IndicesIter<'a> {}

impl From<&Indices> for IndexFormat {
    fn from(indices: &Indices) -> Self {
        match indices {
            Indices::U16(_) => IndexFormat::Uint16,
            Indices::U32(_) => IndexFormat::Uint32,
        }
    }
}
