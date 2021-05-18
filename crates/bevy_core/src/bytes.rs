pub use bevy_derive::Bytes;

// NOTE: we can reexport common traits and methods from bytemuck to avoid requiring dependency most of
// the time, but unfortunately we can't use derive macros that way due to hardcoded path in generated code.
pub use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};

// FIXME: `Bytes` trait doesn't specify the expected encoding format,
// which means types that implement it have to know what format is expected
// and can only implement one encoding at a time.
// TODO: Remove `Bytes` and `FromBytes` in favour of `crevice` crate.

/// Converts the implementing type to bytes by writing them to a given buffer
pub trait Bytes {
    /// Converts the implementing type to bytes by writing them to a given buffer
    fn write_bytes(&self, buffer: &mut [u8]);

    /// The number of bytes that will be written when calling `write_bytes`
    fn byte_len(&self) -> usize;
}

impl<T> Bytes for T
where
    T: Pod,
{
    fn write_bytes(&self, buffer: &mut [u8]) {
        buffer[0..self.byte_len()].copy_from_slice(bytes_of(self))
    }

    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Converts a byte array to `Self`
pub trait FromBytes {
    /// Converts a byte array to `Self`
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl<T> FromBytes for T
where
    T: Pod,
{
    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(
            bytes.len(),
            std::mem::size_of::<T>(),
            "Cannot convert byte slice `&[u8]` to type `{}`. They are not the same size.",
            std::any::type_name::<T>()
        );
        unsafe { bytes.as_ptr().cast::<T>().read_unaligned() }
    }
}

#[cfg(test)]
mod tests {

    use super::{Bytes, FromBytes};
    use bevy_math::{Mat4, Vec2, Vec3, Vec4};

    fn test_round_trip<T: Bytes + FromBytes + std::fmt::Debug + PartialEq>(value: T) {
        let mut bytes = vec![0; value.byte_len()];
        value.write_bytes(&mut bytes);
        let result = T::from_bytes(&bytes);
        assert_eq!(value, result);
    }

    #[test]
    fn test_u32_bytes_round_trip() {
        test_round_trip(123u32);
    }

    #[test]
    fn test_f64_bytes_round_trip() {
        test_round_trip(123f64);
    }

    #[test]
    fn test_vec2_round_trip() {
        test_round_trip(Vec2::new(1.0, 2.0));
    }

    #[test]
    fn test_vec3_round_trip() {
        test_round_trip(Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_vec4_round_trip() {
        test_round_trip(Vec4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_mat4_round_trip() {
        test_round_trip(Mat4::IDENTITY);
    }

    #[test]
    fn test_array_round_trip() {
        test_round_trip([-10i32; 1024]);
        test_round_trip([Vec2::ZERO, Vec2::ONE, Vec2::Y, Vec2::X]);
    }
}
