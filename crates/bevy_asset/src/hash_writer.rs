// XXX TODO: Does this deserve its own file? Where should the file go?

use bevy_platform::hash::{DefaultHasher, FixedHasher};
use core::hash::{BuildHasher, Hasher};

pub(crate) struct HashWriter {
    hasher: DefaultHasher,
}

impl HashWriter {
    pub fn new() -> Self {
        HashWriter {
            hasher: FixedHasher.build_hasher(),
        }
    }

    pub fn finish(self) -> u64 {
        self.hasher.finish()
    }
}

impl std::io::Write for HashWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
