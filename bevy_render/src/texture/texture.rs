use bevy_asset::{Asset, Handle};
use std::fs::File;
use crate::shader::ShaderDefSuffixProvider;

pub enum TextureType {
    Data(Vec<u8>, usize, usize),
    Png(String), // TODO: please rethink this
}

pub struct Texture {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl Asset<TextureType> for Texture {
    fn load(descriptor: TextureType) -> Self {
        let (data, width, height) = match descriptor {
            TextureType::Data(data, width, height) => (data.clone(), width, height),
            TextureType::Png(path) => {
                let decoder = png::Decoder::new(File::open(&path).unwrap());
                let (info, mut reader) = decoder.read_info().unwrap();
                let mut buf = vec![0; info.buffer_size()];
                reader.next_frame(&mut buf).unwrap();
                (buf, info.width as usize, info.height as usize)
            }
        };

        Texture {
            data,
            width,
            height,
        }
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

impl ShaderDefSuffixProvider for Option<Handle<Texture>> {
    fn get_shader_def(&self) -> Option<&'static str> {
        match *self {
            Some(_) => Some(""),
            None => None,
        }
    }
}