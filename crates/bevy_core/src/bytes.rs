use glam::{Mat4, Vec2, Vec3, Vec4};
use zerocopy::AsBytes;

macro_rules! impl_bytes_zerocopy {
    ($ty:tt) => {
        impl Bytes for $ty {
            fn write_bytes(&self, buffer: &mut [u8]) {
                buffer[0..self.byte_len()].copy_from_slice(self.as_bytes())
            }
            fn byte_len(&self) -> usize {
                std::mem::size_of::<Self>()
            }
        }
        
    };
}

pub trait Bytes {
    fn write_bytes(&self, buffer: &mut [u8]);
    fn byte_len(&self) -> usize;
}

impl_bytes_zerocopy!(u8);
impl_bytes_zerocopy!(u16);
impl_bytes_zerocopy!(u32);
impl_bytes_zerocopy!(u64);
impl_bytes_zerocopy!(usize);
impl_bytes_zerocopy!(i8);
impl_bytes_zerocopy!(i16);
impl_bytes_zerocopy!(i32);
impl_bytes_zerocopy!(i64);
impl_bytes_zerocopy!(isize);
impl_bytes_zerocopy!(f32);
impl_bytes_zerocopy!(f64);

impl_bytes_zerocopy!([u8; 2]);
impl_bytes_zerocopy!([u16; 2]);
impl_bytes_zerocopy!([u32; 2]);
impl_bytes_zerocopy!([u64; 2]);
impl_bytes_zerocopy!([usize; 2]);
impl_bytes_zerocopy!([i8; 2]);
impl_bytes_zerocopy!([i16; 2]);
impl_bytes_zerocopy!([i32; 2]);
impl_bytes_zerocopy!([i64; 2]);
impl_bytes_zerocopy!([isize; 2]);
impl_bytes_zerocopy!([f32; 2]);
impl_bytes_zerocopy!([f64; 2]);

impl_bytes_zerocopy!([u8; 3]);
impl_bytes_zerocopy!([u16; 3]);
impl_bytes_zerocopy!([u32; 3]);
impl_bytes_zerocopy!([u64; 3]);
impl_bytes_zerocopy!([usize; 3]);
impl_bytes_zerocopy!([i8; 3]);
impl_bytes_zerocopy!([i16; 3]);
impl_bytes_zerocopy!([i32; 3]);
impl_bytes_zerocopy!([i64; 3]);
impl_bytes_zerocopy!([isize; 3]);
impl_bytes_zerocopy!([f32; 3]);
impl_bytes_zerocopy!([f64; 3]);

impl_bytes_zerocopy!([u8; 4]);
impl_bytes_zerocopy!([u16; 4]);
impl_bytes_zerocopy!([u32; 4]);
impl_bytes_zerocopy!([u64; 4]);
impl_bytes_zerocopy!([usize; 4]);
impl_bytes_zerocopy!([i8; 4]);
impl_bytes_zerocopy!([i16; 4]);
impl_bytes_zerocopy!([i32; 4]);
impl_bytes_zerocopy!([i64; 4]);
impl_bytes_zerocopy!([isize; 4]);
impl_bytes_zerocopy!([f32; 4]);
impl_bytes_zerocopy!([f64; 4]);


impl Bytes for Vec2 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array: [f32; 2] = (*self).into();
        buffer[0..self.byte_len()].copy_from_slice(array.as_bytes())
    }
    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Bytes for Vec3 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array: [f32; 3] = (*self).into();
        buffer[0..self.byte_len()].copy_from_slice(array.as_bytes())
    }
    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Bytes for Vec4 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        let array: [f32; 4] = (*self).into();
        buffer[0..self.byte_len()].copy_from_slice(array.as_bytes())
    }
    fn byte_len(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Bytes for Mat4 {
    fn write_bytes(&self, buffer: &mut [u8]) {
        buffer[0..self.byte_len()].copy_from_slice(self.to_cols_array().as_bytes())
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
