// XXX TODO: Does this deserve its own file? Where should the file go?
// XXX TODO: Annoyingly this has to be pub to support doc tests? Should be pub(crate).

use bevy_platform::hash::{DefaultHasher, FixedHasher};
use core::hash::{BuildHasher, Hasher};

/// A `std::io::Write` implementation that hashes the inputs.
///
/// This is typically used to hash something without having to write a temporary
/// buffer. For example, it can be used to hash the output of a serializer:
///
/// ```
/// # use bevy_asset::hash_writer::HashWriter;
/// # use bevy_platform::hash::FixedHasher;
/// # use std::hash::BuildHasher;
/// # let value = 0u32;
/// let mut hash_writer = HashWriter::default();
/// ron::ser::to_writer(&mut hash_writer, &value);
/// let hash: u64 = hash_writer.finish();
/// ```
pub struct HashWriter {
    hasher: DefaultHasher,
}

impl Default for HashWriter {
    fn default() -> Self {
        HashWriter {
            hasher: FixedHasher.build_hasher(),
        }
    }
}

impl HashWriter {
    pub fn finish(self) -> u64 {
        self.hasher.finish()
    }
}

impl std::io::Write for HashWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        std::dbg!(buf);
        self.hasher.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_writer_equivalence() {
        #[derive(serde::Serialize)]
        struct Test {
            s: &'static str,
            i: u32,
        }

        let value = Test {
            s: "hello",
            i: 1234,
        };

        let mut hash_writer = HashWriter::default();
        ron::ser::to_writer(&mut hash_writer, &value).unwrap();
        let hash: u64 = hash_writer.finish();

        let mut vec = alloc::vec::Vec::<u8>::new();
        ron::ser::to_writer(&mut vec, &value).unwrap();
        std::dbg!(&vec);
        let mut vec_hasher = FixedHasher.build_hasher();
        vec_hasher.write(&vec);
        assert_eq!(hash, vec_hasher.finish());
    }
}
