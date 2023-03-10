use std::iter;

use bevy_render::mesh::morph::{MorphAttributes, VisitAttributes, VisitMorphTargets};
use gltf::{
    accessor::Iter,
    mesh::{util::ReadMorphTargets, Reader},
    Buffer,
};

pub(super) struct PrimitiveMorphTargetAttributes<'s> {
    positions: Option<Iter<'s, [f32; 3]>>,
    normals: Option<Iter<'s, [f32; 3]>>,
    tangents: Option<Iter<'s, [f32; 3]>>,
}
type AllAttributesIter<'s> = (
    Option<Iter<'s, [f32; 3]>>,
    Option<Iter<'s, [f32; 3]>>,
    Option<Iter<'s, [f32; 3]>>,
);

impl<'s> PrimitiveMorphTargetAttributes<'s> {
    fn new((positions, normals, tangents): AllAttributesIter<'s>) -> Self {
        PrimitiveMorphTargetAttributes {
            positions,
            normals,
            tangents,
        }
    }
}
/// A wrapper struct around [`Reader`] to implement [`VisitMorphTargets`].
///
/// The unreasonable parameter list is a side effect of [`Reader`] having
/// an involved type signature itself.
pub(super) struct PrimitiveMorphTargets<'a, 'b, 's, F>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    inner: &'b Reader<'a, 's, F>,
}
impl<'a, 'b, 's, F> PrimitiveMorphTargets<'a, 'b, 's, F>
where
    F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>,
{
    pub fn new(inner: &'b Reader<'a, 's, F>) -> Self {
        PrimitiveMorphTargets { inner }
    }
}
impl<'a, 'b, 's, F: Clone + Fn(Buffer<'a>) -> Option<&'s [u8]>> VisitMorphTargets
    for PrimitiveMorphTargets<'a, 'b, 's, F>
{
    type Visitor = PrimitiveMorphTargetAttributes<'s>;

    type Attributes = iter::Map<
        ReadMorphTargets<'a, 's, F>,
        fn(AllAttributesIter<'s>) -> PrimitiveMorphTargetAttributes<'s>,
    >;

    fn target_count(&self) -> usize {
        self.inner.read_morph_targets().len()
    }

    fn targets(&mut self) -> Self::Attributes {
        self.inner
            .read_morph_targets()
            .map(PrimitiveMorphTargetAttributes::new)
    }
}
impl<'a> VisitAttributes for PrimitiveMorphTargetAttributes<'a> {
    // inline: should allow vectorization in the tight loop that calls this method
    #[inline]
    fn next_attributes(&mut self) -> Option<MorphAttributes> {
        // TODO(#8158): Check beforehand if all entries of an attribute
        // are empty or None, eliminate them from attributes
        const ZERO: [f32; 3] = [0., 0., 0.];
        let query_next = |iter: &mut Option<Iter<_>>| match iter {
            Some(iter) => iter.next().unwrap_or(ZERO),
            None => ZERO,
        };
        Some(MorphAttributes {
            position: query_next(&mut self.positions).into(),
            normal: query_next(&mut self.normals).into(),
            tangent: query_next(&mut self.tangents).into(),
        })
    }
}
