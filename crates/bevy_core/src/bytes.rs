use bevy_math::{Mat4, Vec2, Vec3, Vec4};

pub use bevy_derive::Bytes;

/// Converts the implementing type to bytes by writing them to a given buffer
pub trait Bytes {
    /// Converts the implementing type to bytes by writing them to a given buffer
    fn write_bytes(&self, buffer: &mut [u8]);

    /// The number of bytes that will be written when calling `write_bytes`
    fn byte_len(&self) -> usize;
}

/// A trait that indicates that it is safe to cast the type to a byte array reference.
pub unsafe trait Byteable
where
    Self: Sized,
{
}

impl<T> Bytes for T
where
    T: Byteable,
{
    fn write_bytes(&self, buffer: &mut [u8]) {
        let bytes = self.as_bytes();
        buffer[0..self.byte_len()].copy_from_slice(bytes)
    }

    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Reads the implementing type as a byte array reference
pub trait AsBytes {
    /// Reads the implementing type as a byte array reference
    fn as_bytes(&self) -> &[u8];
}

/// Converts a byte array to `Self`
pub trait FromBytes {
    /// Converts a byte array to `Self`
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl<T> FromBytes for T
where
    T: Byteable + Clone,
{
    fn from_bytes(bytes: &[u8]) -> Self {
        unsafe {
            let byte_ptr = bytes.as_ptr();
            let ptr = byte_ptr as *const Self;
            (*ptr).clone()
        }
    }
}

impl<T> AsBytes for T
where
    T: Byteable,
{
    fn as_bytes(&self) -> &[u8] {
        let len = std::mem::size_of_val(self);
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, len) }
    }
}

impl<'a, T> AsBytes for [T]
where
    T: Byteable,
{
    fn as_bytes(&self) -> &[u8] {
        let len = std::mem::size_of_val(self);
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, len) }
    }
}

unsafe impl<T> Byteable for [T]
where
    Self: Sized,
    T: Byteable,
{
}
unsafe impl<T> Byteable for [T; 2] where T: Byteable {}
unsafe impl<T> Byteable for [T; 3] where T: Byteable {}
unsafe impl<T> Byteable for [T; 4] where T: Byteable {}
unsafe impl<T> Byteable for [T; 16] where T: Byteable {}

unsafe impl Byteable for u8 {}
unsafe impl Byteable for u16 {}
unsafe impl Byteable for u32 {}
unsafe impl Byteable for u64 {}
unsafe impl Byteable for usize {}
unsafe impl Byteable for i8 {}
unsafe impl Byteable for i16 {}
unsafe impl Byteable for i32 {}
unsafe impl Byteable for i64 {}
unsafe impl Byteable for isize {}
unsafe impl Byteable for f32 {}
unsafe impl Byteable for f64 {}
unsafe impl Byteable for Vec2 {}
// NOTE: Vec3 actually takes up the size of 4 floats / 16 bytes due to SIMD. This is actually convenient because GLSL
// uniform buffer objects pad Vec3s to be 16 bytes.
unsafe impl Byteable for Vec3 {}
unsafe impl Byteable for Vec4 {}

impl Bytes for Mat4 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array = self.to_cols_array();
        array.write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl FromBytes for Mat4 {
    fn from_bytes(bytes: &[u8]) -> Self {
        let array = <[f32; 16]>::from_bytes(bytes);
        Mat4::from_cols_array(&array)
    }
}

impl<T> Bytes for Option<T>
where
    T: Bytes,
{
    fn write_bytes(&self, buffer: &mut [u8]) {
        if let Some(val) = self {
            val.write_bytes(buffer)
        }
    }

    fn byte_len(&self) -> usize {
        self.as_ref().map_or(0, |val| val.byte_len())
    }
}

impl<T> FromBytes for Option<T>
where
    T: FromBytes,
{
    fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            None
        } else {
            Some(T::from_bytes(bytes))
        }
    }
}

impl<T> Bytes for Vec<T>
where
    T: Sized + Byteable,
{
    fn write_bytes(&self, buffer: &mut [u8]) {
        let bytes = self.as_slice().as_bytes();
        buffer[0..self.byte_len()].copy_from_slice(bytes)
    }

    fn byte_len(&self) -> usize {
        self.as_slice().as_bytes().len()
    }
}

impl<T> FromBytes for Vec<T>
where
    T: Sized + Clone + Byteable,
{
    fn from_bytes(bytes: &[u8]) -> Self {
        unsafe {
            let byte_ptr = bytes.as_ptr() as *const T;
            let len = bytes.len() / std::mem::size_of::<T>();
            let slice = core::slice::from_raw_parts::<T>(byte_ptr, len);
            slice.to_vec()
        }
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
    fn test_vec_bytes_round_trip() {
        test_round_trip(vec![1u32, 2u32, 3u32]);
    }

    #[test]
    fn test_option_bytes_round_trip() {
        test_round_trip(Some(123u32));
        test_round_trip(Option::<u32>::None);
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
        test_round_trip(Mat4::identity());
    }
}
