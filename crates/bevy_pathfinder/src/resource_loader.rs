use pathfinder_resources::ResourceLoader;

pub struct BevyResourceLoader {

}

impl BevyResourceLoader {
    pub fn new() -> Self {
        BevyResourceLoader {}
    }
}

const AREA_LUT: &'static [u8] = include_bytes!("resources/area-lut.png");
const GAMMA_LUT: &'static [u8] = include_bytes!("resources/gamma-lut.png");

impl ResourceLoader for BevyResourceLoader {
    fn slurp(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        match path {
            "textures/area-lut.png" => Ok(AREA_LUT.to_vec()),
            "textures/gamma-lut.png" => Ok(GAMMA_LUT.to_vec()),
            _ => panic!("failed to find resource {}", path),
        }
    }
}
