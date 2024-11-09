use bevy_math::Affine2;

pub trait TextureTransformExt {
    fn convert_texture_transform_to_affine2(&self) -> Affine2;
}

impl TextureTransformExt for gltf::texture::TextureTransform<'_> {
    fn convert_texture_transform_to_affine2(&self) -> Affine2 {
        Affine2::from_scale_angle_translation(
            self.scale().into(),
            -self.rotation(),
            self.offset().into(),
        )
    }
}
