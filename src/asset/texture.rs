use crate::asset::Asset;

pub enum TextureType {
    Data(Vec<u8>),
}

pub struct Texture {
    pub data: Vec<u8>,
}

impl Asset<TextureType> for Texture {
    fn load(descriptor: TextureType) -> Self {
        let data = match descriptor {
            TextureType::Data(data) => data.clone(),
        };

        Texture { data: data }
    }
}

pub fn create_texels(size: usize) -> Vec<u8> {
    use std::iter;

    (0..size * size)
        .flat_map(|id| {
            // get high five for recognizing this ;)
            let cx = 3.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
            let cy = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
            let (mut x, mut y, mut count) = (cx, cy, 0);
            while count < 0xFF && x * x + y * y < 4.0 {
                let old_x = x;
                x = x * x - y * y + cx;
                y = 2.0 * old_x * y + cy;
                count += 1;
            }
            iter::once(0xFF - (count * 5) as u8)
                .chain(iter::once(0xFF - (count * 15) as u8))
                .chain(iter::once(0xFF - (count * 50) as u8))
                .chain(iter::once(1))
        })
        .collect()
}
