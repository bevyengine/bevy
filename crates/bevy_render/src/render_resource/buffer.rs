bitflags::bitflags! {
    #[repr(transparent)]
    #[cfg_attr(feature = "trace", derive(Serialize))]
    #[cfg_attr(feature = "replay", derive(Deserialize))]
    pub struct BufferUsage: u32 {
        const MAP_READ = 1;
        const MAP_WRITE = 2;
        const COPY_SRC = 4;
        const COPY_DST = 8;
        const INDEX = 16;
        const VERTEX = 32;
        const UNIFORM = 64;
        const STORAGE = 128;
        const INDIRECT = 256;
    }
}
