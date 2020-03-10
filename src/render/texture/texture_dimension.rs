#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum TextureViewDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

impl From<TextureViewDimension> for wgpu::TextureViewDimension {
    fn from(dimension: TextureViewDimension) -> Self {
        match dimension {
            TextureViewDimension::D1 => wgpu::TextureViewDimension::D1,
            TextureViewDimension::D2 => wgpu::TextureViewDimension::D2,
            TextureViewDimension::D2Array => wgpu::TextureViewDimension::D2Array,
            TextureViewDimension::Cube => wgpu::TextureViewDimension::Cube,
            TextureViewDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
            TextureViewDimension::D3 => wgpu::TextureViewDimension::D3,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}

impl From<TextureDimension> for wgpu::TextureDimension {
    fn from(dimension: TextureDimension) -> Self {
        match dimension {
            TextureDimension::D1 => wgpu::TextureDimension::D1,
            TextureDimension::D2 => wgpu::TextureDimension::D2,
            TextureDimension::D3 => wgpu::TextureDimension::D3,
        }
    }
}
