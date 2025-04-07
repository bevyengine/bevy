use crate::{Image, TextureAccessError, TextureFormatPixelInfo};
use bevy_color::{Color, ColorToComponents, Gray, LinearRgba, Srgba, Xyza};
use bevy_math::UVec3;
use bytes::BytesMut;
use core::mem;
use wgpu_types::{TextureDimension, TextureFormat};

impl Image {
    /// If not readonly, returns a mutable version of the underlying data.
    pub fn as_mut(&mut self) -> Option<ImageMut> {
        if self.is_readonly() {
            None
        } else {
            let data = mem::take(&mut self.data).into();
            Some(ImageMut { image: self, data })
        }
    }

    /// Returns a mutable version of the underlying data,
    /// duplicates the underlying data if readonly.
    pub fn to_mut(&mut self) -> ImageMut {
        let data = mem::take(&mut self.data).into();
        ImageMut { image: self, data }
    }
}

/// An [`Image`] with mutable underlying data.
///
/// [`ImageMut::data`] will be copied back to [`ImageMut::image`] on drop.
pub struct ImageMut<'t> {
    pub image: &'t mut Image,
    pub data: BytesMut,
}

impl Drop for ImageMut<'_> {
    fn drop(&mut self) {
        self.image.data = mem::take(&mut self.data).into();
    }
}

impl ImageMut<'_> {
    /// Get a reference to the data bytes where a specific pixel's value is stored
    #[inline(always)]
    pub fn pixel_bytes(&self, coords: UVec3) -> Option<&[u8]> {
        let len = self.image.texture_descriptor.format.pixel_size();
        let offset = self.image.pixel_data_offset(coords);
        offset.map(|start| &self.data[start..(start + len)])
    }

    /// Get a mutable reference to the data bytes where a specific pixel's value is stored
    #[inline(always)]
    pub fn pixel_bytes_mut(&mut self, coords: UVec3) -> Option<&mut [u8]> {
        let len = self.image.texture_descriptor.format.pixel_size();
        let offset = self.image.pixel_data_offset(coords);
        offset.map(|start| &mut self.data[start..(start + len)])
    }

    /// Read the color of a specific pixel (1D texture).
    ///
    /// See [`get_color_at`](Self::get_color_at) for more details.
    #[inline(always)]
    pub fn get_color_at_1d(&self, x: u32) -> Result<Color, TextureAccessError> {
        if self.image.texture_descriptor.dimension != TextureDimension::D1 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.get_color_at_internal(UVec3::new(x, 0, 0))
    }

    /// Read the color of a specific pixel (2D texture).
    ///
    /// This function will find the raw byte data of a specific pixel and
    /// decode it into a user-friendly [`Color`] struct for you.
    ///
    /// Supports many of the common [`TextureFormat`]s:
    ///  - RGBA/BGRA 8-bit unsigned integer, both sRGB and Linear
    ///  - 16-bit and 32-bit unsigned integer
    ///  - 16-bit and 32-bit float
    ///
    /// Be careful: as the data is converted to [`Color`] (which uses `f32` internally),
    /// there may be issues with precision when using non-f32 [`TextureFormat`]s.
    /// If you read a value you previously wrote using `set_color_at`, it will not match.
    /// If you are working with a 32-bit integer [`TextureFormat`], the value will be
    /// inaccurate (as `f32` does not have enough bits to represent it exactly).
    ///
    /// Single channel (R) formats are assumed to represent grayscale, so the value
    /// will be copied to all three RGB channels in the resulting [`Color`].
    ///
    /// Other [`TextureFormat`]s are unsupported, such as:
    ///  - block-compressed formats
    ///  - non-byte-aligned formats like 10-bit
    ///  - signed integer formats
    #[inline(always)]
    pub fn get_color_at(&self, x: u32, y: u32) -> Result<Color, TextureAccessError> {
        if self.image.texture_descriptor.dimension != TextureDimension::D2 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.get_color_at_internal(UVec3::new(x, y, 0))
    }

    /// Read the color of a specific pixel (2D texture with layers or 3D texture).
    ///
    /// See [`get_color_at`](Self::get_color_at) for more details.
    #[inline(always)]
    pub fn get_color_at_3d(&self, x: u32, y: u32, z: u32) -> Result<Color, TextureAccessError> {
        match (
            self.image.texture_descriptor.dimension,
            self.image.texture_descriptor.size.depth_or_array_layers,
        ) {
            (TextureDimension::D3, _) | (TextureDimension::D2, 2..) => {
                self.get_color_at_internal(UVec3::new(x, y, z))
            }
            _ => Err(TextureAccessError::WrongDimension),
        }
    }

    /// Change the color of a specific pixel (1D texture).
    ///
    /// See [`set_color_at`](Self::set_color_at) for more details.
    #[inline(always)]
    pub fn set_color_at_1d(&mut self, x: u32, color: Color) -> Result<(), TextureAccessError> {
        if self.image.texture_descriptor.dimension != TextureDimension::D1 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.set_color_at_internal(UVec3::new(x, 0, 0), color)
    }

    /// Change the color of a specific pixel (2D texture).
    ///
    /// This function will find the raw byte data of a specific pixel and
    /// change it according to a [`Color`] you provide. The [`Color`] struct
    /// will be encoded into the [`Image`]'s [`TextureFormat`].
    ///
    /// Supports many of the common [`TextureFormat`]s:
    ///  - RGBA/BGRA 8-bit unsigned integer, both sRGB and Linear
    ///  - 16-bit and 32-bit unsigned integer (with possibly-limited precision, as [`Color`] uses `f32`)
    ///  - 16-bit and 32-bit float
    ///
    /// Be careful: writing to non-f32 [`TextureFormat`]s is lossy! The data has to be converted,
    /// so if you read it back using `get_color_at`, the `Color` you get will not equal the value
    /// you used when writing it using this function.
    ///
    /// For R and RG formats, only the respective values from the linear RGB [`Color`] will be used.
    ///
    /// Other [`TextureFormat`]s are unsupported, such as:
    ///  - block-compressed formats
    ///  - non-byte-aligned formats like 10-bit
    ///  - signed integer formats
    #[inline(always)]
    pub fn set_color_at(&mut self, x: u32, y: u32, color: Color) -> Result<(), TextureAccessError> {
        if self.image.texture_descriptor.dimension != TextureDimension::D2 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.set_color_at_internal(UVec3::new(x, y, 0), color)
    }

    /// Change the color of a specific pixel (2D texture with layers or 3D texture).
    ///
    /// See [`set_color_at`](Self::set_color_at) for more details.
    #[inline(always)]
    pub fn set_color_at_3d(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        color: Color,
    ) -> Result<(), TextureAccessError> {
        match (
            self.image.texture_descriptor.dimension,
            self.image.texture_descriptor.size.depth_or_array_layers,
        ) {
            (TextureDimension::D3, _) | (TextureDimension::D2, 2..) => {
                self.set_color_at_internal(UVec3::new(x, y, z), color)
            }
            _ => Err(TextureAccessError::WrongDimension),
        }
    }

    #[inline(always)]
    fn get_color_at_internal(&self, coords: UVec3) -> Result<Color, TextureAccessError> {
        let Some(bytes) = self.pixel_bytes(coords) else {
            return Err(TextureAccessError::OutOfBounds {
                x: coords.x,
                y: coords.y,
                z: coords.z,
            });
        };

        // NOTE: GPUs are always Little Endian.
        // Make sure to respect that when we create color values from bytes.
        match self.image.texture_descriptor.format {
            TextureFormat::Rgba8UnormSrgb => Ok(Color::srgba(
                bytes[0] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[2] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8Uint => Ok(Color::linear_rgba(
                bytes[0] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[2] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Bgra8UnormSrgb => Ok(Color::srgba(
                bytes[2] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[0] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Bgra8Unorm => Ok(Color::linear_rgba(
                bytes[2] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[0] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Rgba32Float => Ok(Color::linear_rgba(
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            )),
            TextureFormat::Rgba16Float => Ok(Color::linear_rgba(
                half::f16::from_le_bytes([bytes[0], bytes[1]]).to_f32(),
                half::f16::from_le_bytes([bytes[2], bytes[3]]).to_f32(),
                half::f16::from_le_bytes([bytes[4], bytes[5]]).to_f32(),
                half::f16::from_le_bytes([bytes[6], bytes[7]]).to_f32(),
            )),
            TextureFormat::Rgba16Unorm | TextureFormat::Rgba16Uint => {
                let (r, g, b, a) = (
                    u16::from_le_bytes([bytes[0], bytes[1]]),
                    u16::from_le_bytes([bytes[2], bytes[3]]),
                    u16::from_le_bytes([bytes[4], bytes[5]]),
                    u16::from_le_bytes([bytes[6], bytes[7]]),
                );
                Ok(Color::linear_rgba(
                    // going via f64 to avoid rounding errors with large numbers and division
                    (r as f64 / u16::MAX as f64) as f32,
                    (g as f64 / u16::MAX as f64) as f32,
                    (b as f64 / u16::MAX as f64) as f32,
                    (a as f64 / u16::MAX as f64) as f32,
                ))
            }
            TextureFormat::Rgba32Uint => {
                let (r, g, b, a) = (
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                    u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                    u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                    u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
                );
                Ok(Color::linear_rgba(
                    // going via f64 to avoid rounding errors with large numbers and division
                    (r as f64 / u32::MAX as f64) as f32,
                    (g as f64 / u32::MAX as f64) as f32,
                    (b as f64 / u32::MAX as f64) as f32,
                    (a as f64 / u32::MAX as f64) as f32,
                ))
            }
            // assume R-only texture format means grayscale (linear)
            // copy value to all of RGB in Color
            TextureFormat::R8Unorm | TextureFormat::R8Uint => {
                let x = bytes[0] as f32 / u8::MAX as f32;
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::R16Unorm | TextureFormat::R16Uint => {
                let x = u16::from_le_bytes([bytes[0], bytes[1]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let x = (x as f64 / u16::MAX as f64) as f32;
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::R32Uint => {
                let x = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let x = (x as f64 / u32::MAX as f64) as f32;
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::R16Float => {
                let x = half::f16::from_le_bytes([bytes[0], bytes[1]]).to_f32();
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::R32Float => {
                let x = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok(Color::linear_rgb(x, x, x))
            }
            TextureFormat::Rg8Unorm | TextureFormat::Rg8Uint => {
                let r = bytes[0] as f32 / u8::MAX as f32;
                let g = bytes[1] as f32 / u8::MAX as f32;
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            TextureFormat::Rg16Unorm | TextureFormat::Rg16Uint => {
                let r = u16::from_le_bytes([bytes[0], bytes[1]]);
                let g = u16::from_le_bytes([bytes[2], bytes[3]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let r = (r as f64 / u16::MAX as f64) as f32;
                let g = (g as f64 / u16::MAX as f64) as f32;
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            TextureFormat::Rg32Uint => {
                let r = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let g = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let r = (r as f64 / u32::MAX as f64) as f32;
                let g = (g as f64 / u32::MAX as f64) as f32;
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            TextureFormat::Rg16Float => {
                let r = half::f16::from_le_bytes([bytes[0], bytes[1]]).to_f32();
                let g = half::f16::from_le_bytes([bytes[2], bytes[3]]).to_f32();
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            TextureFormat::Rg32Float => {
                let r = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let g = f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                Ok(Color::linear_rgb(r, g, 0.0))
            }
            _ => Err(TextureAccessError::UnsupportedTextureFormat(
                self.image.texture_descriptor.format,
            )),
        }
    }

    #[inline(always)]
    fn set_color_at_internal(
        &mut self,
        coords: UVec3,
        color: Color,
    ) -> Result<(), TextureAccessError> {
        let format = self.image.texture_descriptor.format;

        let Some(bytes) = self.pixel_bytes_mut(coords) else {
            return Err(TextureAccessError::OutOfBounds {
                x: coords.x,
                y: coords.y,
                z: coords.z,
            });
        };

        // NOTE: GPUs are always Little Endian.
        // Make sure to respect that when we convert color values to bytes.
        match format {
            TextureFormat::Rgba8UnormSrgb => {
                let [r, g, b, a] = Srgba::from(color).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (b * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8Uint => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (b * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Bgra8UnormSrgb => {
                let [r, g, b, a] = Srgba::from(color).to_f32_array();
                bytes[0] = (b * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (r * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Bgra8Unorm => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                bytes[0] = (b * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
                bytes[2] = (r * u8::MAX as f32) as u8;
                bytes[3] = (a * u8::MAX as f32) as u8;
            }
            TextureFormat::Rgba16Float => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                bytes[0..2].copy_from_slice(&half::f16::to_le_bytes(half::f16::from_f32(r)));
                bytes[2..4].copy_from_slice(&half::f16::to_le_bytes(half::f16::from_f32(g)));
                bytes[4..6].copy_from_slice(&half::f16::to_le_bytes(half::f16::from_f32(b)));
                bytes[6..8].copy_from_slice(&half::f16::to_le_bytes(half::f16::from_f32(a)));
            }
            TextureFormat::Rgba32Float => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                bytes[0..4].copy_from_slice(&f32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&f32::to_le_bytes(g));
                bytes[8..12].copy_from_slice(&f32::to_le_bytes(b));
                bytes[12..16].copy_from_slice(&f32::to_le_bytes(a));
            }
            TextureFormat::Rgba16Unorm | TextureFormat::Rgba16Uint => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                let [r, g, b, a] = [
                    (r * u16::MAX as f32) as u16,
                    (g * u16::MAX as f32) as u16,
                    (b * u16::MAX as f32) as u16,
                    (a * u16::MAX as f32) as u16,
                ];
                bytes[0..2].copy_from_slice(&u16::to_le_bytes(r));
                bytes[2..4].copy_from_slice(&u16::to_le_bytes(g));
                bytes[4..6].copy_from_slice(&u16::to_le_bytes(b));
                bytes[6..8].copy_from_slice(&u16::to_le_bytes(a));
            }
            TextureFormat::Rgba32Uint => {
                let [r, g, b, a] = LinearRgba::from(color).to_f32_array();
                let [r, g, b, a] = [
                    (r * u32::MAX as f32) as u32,
                    (g * u32::MAX as f32) as u32,
                    (b * u32::MAX as f32) as u32,
                    (a * u32::MAX as f32) as u32,
                ];
                bytes[0..4].copy_from_slice(&u32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&u32::to_le_bytes(g));
                bytes[8..12].copy_from_slice(&u32::to_le_bytes(b));
                bytes[12..16].copy_from_slice(&u32::to_le_bytes(a));
            }
            TextureFormat::R8Unorm | TextureFormat::R8Uint => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
            }
            TextureFormat::R16Unorm | TextureFormat::R16Uint => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                let r = (r * u16::MAX as f32) as u16;
                bytes[0..2].copy_from_slice(&u16::to_le_bytes(r));
            }
            TextureFormat::R32Uint => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                // go via f64 to avoid imprecision
                let r = (r as f64 * u32::MAX as f64) as u32;
                bytes[0..4].copy_from_slice(&u32::to_le_bytes(r));
            }
            TextureFormat::R16Float => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                let x = half::f16::from_f32(r);
                bytes[0..2].copy_from_slice(&half::f16::to_le_bytes(x));
            }
            TextureFormat::R32Float => {
                // Convert to grayscale with minimal loss if color is already gray
                let linear = LinearRgba::from(color);
                let luminance = Xyza::from(linear).y;
                let [r, _, _, _] = LinearRgba::gray(luminance).to_f32_array();
                bytes[0..4].copy_from_slice(&f32::to_le_bytes(r));
            }
            TextureFormat::Rg8Unorm | TextureFormat::Rg8Uint => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                bytes[0] = (r * u8::MAX as f32) as u8;
                bytes[1] = (g * u8::MAX as f32) as u8;
            }
            TextureFormat::Rg16Unorm | TextureFormat::Rg16Uint => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                let r = (r * u16::MAX as f32) as u16;
                let g = (g * u16::MAX as f32) as u16;
                bytes[0..2].copy_from_slice(&u16::to_le_bytes(r));
                bytes[2..4].copy_from_slice(&u16::to_le_bytes(g));
            }
            TextureFormat::Rg32Uint => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                // go via f64 to avoid imprecision
                let r = (r as f64 * u32::MAX as f64) as u32;
                let g = (g as f64 * u32::MAX as f64) as u32;
                bytes[0..4].copy_from_slice(&u32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&u32::to_le_bytes(g));
            }
            TextureFormat::Rg16Float => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                bytes[0..2].copy_from_slice(&half::f16::to_le_bytes(half::f16::from_f32(r)));
                bytes[2..4].copy_from_slice(&half::f16::to_le_bytes(half::f16::from_f32(g)));
            }
            TextureFormat::Rg32Float => {
                let [r, g, _, _] = LinearRgba::from(color).to_f32_array();
                bytes[0..4].copy_from_slice(&f32::to_le_bytes(r));
                bytes[4..8].copy_from_slice(&f32::to_le_bytes(g));
            }
            _ => {
                return Err(TextureAccessError::UnsupportedTextureFormat(
                    self.image.texture_descriptor.format,
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bevy_asset::RenderAssetUsages;
    use bevy_color::Color;
    use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

    use crate::Image;

    #[test]
    fn get_set_pixel_2d_with_layers() {
        let mut image = Image::new_fill(
            Extent3d {
                width: 5,
                height: 10,
                depth_or_array_layers: 3,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD,
        );
        let mut image_mut = image.as_mut().unwrap();
        image_mut.set_color_at_3d(0, 0, 0, Color::WHITE).unwrap();
        assert!(matches!(
            image_mut.get_color_at_3d(0, 0, 0),
            Ok(Color::WHITE)
        ));
        image_mut.set_color_at_3d(2, 3, 1, Color::WHITE).unwrap();
        assert!(matches!(
            image_mut.get_color_at_3d(2, 3, 1),
            Ok(Color::WHITE)
        ));
        image_mut.set_color_at_3d(4, 9, 2, Color::WHITE).unwrap();
        assert!(matches!(
            image_mut.get_color_at_3d(4, 9, 2),
            Ok(Color::WHITE)
        ));
        drop(image_mut);
        assert!(matches!(image.get_color_at_3d(0, 0, 0), Ok(Color::WHITE)));
        assert!(matches!(image.get_color_at_3d(2, 3, 1), Ok(Color::WHITE)));
        assert!(matches!(image.get_color_at_3d(4, 9, 2), Ok(Color::WHITE)));
    }
}
