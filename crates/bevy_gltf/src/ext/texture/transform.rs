use bevy_math::Affine2;
use gltf::texture::TextureTransform;

pub trait TextureTransformExt {
    fn to_affine2(self) -> Affine2;
}

impl TextureTransformExt for TextureTransform<'_> {
    fn to_affine2(self) -> Affine2 {
        Affine2::from_scale_angle_translation(
            self.scale().into(),
            -self.rotation(),
            self.offset().into(),
        )
    }
}
