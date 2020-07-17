use uuid::Uuid;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct BufferId(Uuid);

impl BufferId {
    pub fn new() -> Self {
        BufferId(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BufferInfo {
    pub size: usize,
    pub buffer_usage: BufferUsage,
    pub mapped_at_creation: bool,
}

impl Default for BufferInfo {
    fn default() -> Self {
        BufferInfo {
            size: 0,
            buffer_usage: BufferUsage::empty(),
            mapped_at_creation: false,
        }
    }
}

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
