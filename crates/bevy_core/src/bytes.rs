use glam::{Mat4, Vec4, Vec3, Vec2};
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

impl GetBytes for Vec3 {
    fn get_bytes(&self) -> Vec<u8> {
        let vec3_array: [f32; 3] = (*self).into();
        vec3_array.as_bytes().into()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_ref().as_bytes())
    }
}

impl GetBytes for Vec2 {
    fn get_bytes(&self) -> Vec<u8> {
        let vec2_array: [f32; 2] = (*self).into();
        vec2_array.as_bytes().into()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_ref().as_bytes())
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

impl GetBytes for Mat4 {
    fn get_bytes(&self) -> Vec<u8> {
        self.as_ref()
            .as_bytes()
            .iter()
            .cloned()
            .collect::<Vec<u8>>()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        Some(self.as_ref().as_bytes())
    }
}

impl<T> GetBytes for Option<T>
where
    T: GetBytes,
{
    fn get_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        self.as_ref()
            .and_then(|get_bytes| get_bytes.get_bytes_ref())
    }
}
