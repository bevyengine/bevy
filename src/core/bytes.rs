use crate::{asset::Handle, math::Vec4, render::texture::Texture};
use zerocopy::AsBytes;

pub trait GetBytes {
    fn get_bytes(&self) -> Vec<u8>;
    fn get_bytes_ref(&self) -> Option<&[u8]>;
}

impl GetBytes for f32 {
    fn get_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_bytes())
    }
}

impl GetBytes for [f32; 2] {
    fn get_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_bytes())
    }
}

impl GetBytes for [f32; 3] {
    fn get_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_bytes())
    }
}

impl GetBytes for [f32; 4] {
    fn get_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_bytes())
    }
}

impl GetBytes for Vec4 {
    fn get_bytes(&self) -> Vec<u8> {
        let vec4_array: [f32; 4] = (*self).into();
        vec4_array.as_bytes().into()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_ref().as_bytes())
    }
}

impl GetBytes for Handle<Texture> {
    fn get_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        None
    }
}
