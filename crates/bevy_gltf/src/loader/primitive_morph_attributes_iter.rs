use bevy_math::Vec3;
use bevy_render::mesh::morph::MorphAttributes;

use gltf::accessor::Iter;

pub(crate) struct PrimitiveMorphAttributesIter<'s>(
    pub  (
        Option<Iter<'s, [f32; 3]>>,
        Option<Iter<'s, [f32; 3]>>,
        Option<Iter<'s, [f32; 3]>>,
    ),
);

impl<'s> Iterator for PrimitiveMorphAttributesIter<'s> {
    type Item = MorphAttributes;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.0 .0.as_mut().and_then(Iterator::next);
        let normal = self.0 .1.as_mut().and_then(Iterator::next);
        let tangent = self.0 .2.as_mut().and_then(Iterator::next);
        if position.is_none() && normal.is_none() && tangent.is_none() {
            return None;
        }

        Some(MorphAttributes {
            position: position.map(Into::into).unwrap_or(Vec3::ZERO),
            normal: normal.map(Into::into).unwrap_or(Vec3::ZERO),
            tangent: tangent.map(Into::into).unwrap_or(Vec3::ZERO),
        })
    }
}
