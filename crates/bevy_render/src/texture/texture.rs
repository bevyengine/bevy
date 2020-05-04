use crate::shader::ShaderDefSuffixProvider;
use bevy_asset::{Asset, Handle};
use std::fs::File;

pub const TEXTURE_ASSET_INDEX: usize = 0;
pub const SAMPLER_ASSET_INDEX: usize = 1;
pub enum TextureType {
    Data(Vec<u8>, usize, usize),
    Png(String), // TODO: please rethink this
}

pub struct Texture {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl Texture {
    pub fn aspect(&self) -> f32 {
        self.height as f32 / self.width as f32
    } 
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

impl ShaderDefSuffixProvider for Option<Handle<Texture>> {
    fn get_shader_def(&self) -> Option<&'static str> {
        match *self {
            Some(_) => Some(""),
            None => None,
        }
    }
}
