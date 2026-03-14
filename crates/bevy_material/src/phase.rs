/// Which phase a material renders in.
#[derive(Clone, Copy, Default)]
pub enum RenderPhaseType {
    #[default]
    Opaque,
    AlphaMask,
    Transmissive,
    Transparent,
}
