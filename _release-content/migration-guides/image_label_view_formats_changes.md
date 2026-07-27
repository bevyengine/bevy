---
title: The types of `label` and `view_formats` in `Image` and `ImageSamplerDescriptor` are changed
pull_requests: [24056]
---

The types of `Image::texture_descriptor`, `Image::texture_view_descriptor` and `ImageSamplerDescriptor::label` are changed.

```rust
// BEFORE
pub struct Image {
    pub texture_descriptor: TextureDescriptor<Option<&'static str>, &'static [TextureFormat]>,
    pub texture_view_descriptor: Option<TextureViewDescriptor<Option<&'static str>>>,
    // ...
}

pub struct ImageSamplerDescriptor {
    pub label: Option<String>,
    // ...
}

// AFTER
pub type ImageTextureViewFormats = SmallVec<[TextureFormat; 1]>;
pub type ImageTextureDescriptor =
    TextureDescriptor<Option<Cow<'static, str>>, ImageTextureViewFormats>;
pub type ImageTextureViewDescriptor = TextureViewDescriptor<Option<Cow<'static, str>>>;

pub struct Image {
    pub texture_descriptor: ImageTextureDescriptor,
    pub texture_view_descriptor: Option<ImageTextureViewDescriptor>,
    // ...
}

pub struct ImageSamplerDescriptor {
    pub label: Option<Cow<'static, str>>,
    // ...
}
```

To use `image.texture_descriptor` and `image.texture_view_descriptor` with wgpu, you can use `ImageDescriptorAsWgpu::as_wgpu`:

```rust
let texture = render_device.create_texture(&image.texture_descriptor.as_wgpu());
let texture_view = image.texture_view_descriptor.as_ref().map(|desc| texture.create_view(&desc.as_wgpu()))
```
