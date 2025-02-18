use {
    crate::ext::texture::TextureExt,
    bevy_asset::{Handle, LoadContext},
    bevy_image::Image,
    gltf::Document,
};

pub trait InfoExt {
    fn texture_handle(&self, document: &Document, load_context: &mut LoadContext) -> Handle<Image>;
}

impl InfoExt for gltf::json::texture::Info {
    /// Given a [`gltf::json::texture::Info`], returns the handle of the texture that this
    /// refers to.
    ///
    /// This is a low-level function only used when the [`gltf`] crate has no support
    /// for an extension, forcing us to parse its texture references manually.
    fn texture_handle(&self, document: &Document, load_context: &mut LoadContext) -> Handle<Image> {
        let texture = document
            .textures()
            .nth(self.index.value())
            .expect("Texture info references a nonexistent texture");
        texture.handle(load_context)
    }
}
