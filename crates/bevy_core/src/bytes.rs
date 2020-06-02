use glam::{Mat4, Vec2, Vec3, Vec4};
pub trait Bytes {
    fn write_bytes(&self, buffer: &mut [u8]);
    fn byte_len(&self) -> usize;
}

pub unsafe trait Byteable where Self: Sized {}

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

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
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

unsafe impl<T> Byteable for [T] where Self: Sized {}
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

impl Bytes for Vec2 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array: [f32; 2] = (*self).into();
        array.write_bytes(buffer);
    }
    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Bytes for Vec3 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array: [f32; 3] = (*self).into();
        array.write_bytes(buffer);
    }
    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Bytes for Vec4 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array: [f32; 4] = (*self).into();
        array.write_bytes(buffer);
    }
    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Bytes for Mat4 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array = self.to_cols_array();
        array.write_bytes(buffer);
    }
    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
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
