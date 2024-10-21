pub trait TexInfoExt {
    fn texture(&self) -> gltf::texture::Texture;
    fn texture_transform(&self) -> Option<gltf::texture::TextureTransform>;
    fn tex_coord(&self) -> u32;
}

impl TexInfoExt for gltf::texture::Info<'_> {
    fn texture(&self) -> gltf::texture::Texture {
        self.texture()
    }

    fn texture_transform(&self) -> Option<gltf::texture::TextureTransform> {
        self.texture_transform()
    }

    fn tex_coord(&self) -> u32 {
        self.tex_coord()
    }
}

impl TexInfoExt for gltf::material::NormalTexture<'_> {
    fn texture(&self) -> gltf::texture::Texture {
        self.texture()
    }

    fn texture_transform(&self) -> Option<gltf::texture::TextureTransform> {
        None
    }

    fn tex_coord(&self) -> u32 {
        self.tex_coord()
    }
}

impl TexInfoExt for gltf::material::OcclusionTexture<'_> {
    fn texture(&self) -> gltf::texture::Texture {
        self.texture()
    }

    fn texture_transform(&self) -> Option<gltf::texture::TextureTransform> {
        None
    }

    fn tex_coord(&self) -> u32 {
        self.tex_coord()
    }
}
