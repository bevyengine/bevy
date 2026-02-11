---
title: "`Image::pixel_bytes` and `Image::pixel_data_offset` now return `Result`"
pull_requests: [22908]
---

`Image::pixel_bytes`, `Image::pixel_bytes_mut`, and `Image::pixel_data_offset` now return `Result<..., TextureAccessError>` instead of `Option<...>`.

Previously, these methods returned `Option` and would silently return `None` for both out-of-bounds access and unsupported texture formats (such as compressed textures). This caused error information to be lost, making it impossible for callers to distinguish between these different failure cases.

Now, these methods properly propagate `TextureAccessError`:
- `TextureAccessError::OutOfBounds` for coordinates outside the image bounds
- `TextureAccessError::UnsupportedTextureFormat` for compressed or unsupported texture formats
- `Ok(None)` (for `pixel_bytes`/`pixel_bytes_mut`) only when the image data is not initialized

**Migration**

Update any code using these methods to handle the `Result` return type:

```rust
// Before
if let Some(bytes) = image.pixel_bytes(coords) {
    // use bytes
}

// After
match image.pixel_bytes(coords) {
    Ok(Some(bytes)) => {
        // use bytes
    }
    Ok(None) => {
        // image data not initialized
    }
    Err(TextureAccessError::OutOfBounds { .. }) => {
        // handle out of bounds
    }
    Err(TextureAccessError::UnsupportedTextureFormat(format)) => {
        // handle compressed/unsupported format
    }
}
```

Or if you only care about the success case with initialized data:

```rust
// Before
let bytes = image.pixel_bytes_mut(coords).unwrap();

// After  
let bytes = image.pixel_bytes_mut(coords).unwrap().unwrap();
```
