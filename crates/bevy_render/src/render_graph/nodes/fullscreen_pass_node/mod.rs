pub mod node;

pub mod shaders {
    pub const VERTEX_SHADER: &str = include_str!("fullscreen.vert");
    pub const NOOP_SHADER: &str = include_str!("noop.frag");
}
