use std::convert::{TryFrom, TryInto};

/// A enum representing a type of file.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    Directory,
    File,
}

impl FileType {
    pub const fn is_dir(&self) -> bool {
        (*self as isize) == (Self::Directory as isize)
    }

    pub const fn is_file(&self) -> bool {
        (*self as isize) == (Self::File as isize)
    }
}

impl TryFrom<std::fs::FileType> for FileType {
    type Error = std::io::Error;

    fn try_from(file_type: std::fs::FileType) -> Result<Self, Self::Error> {
        if file_type.is_dir() {
            Ok(Self::Directory)
        } else if file_type.is_file() {
            Ok(Self::File)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unknown file type",
            ))
        }
    }
}

/// Metadata information about a file.
///
/// This structure is returned from the [`AssetIo::get_metadata`] method.
#[derive(Debug, Clone)]
pub struct Metadata {
    pub file_type: FileType,
}

impl Metadata {
    pub const fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    pub const fn is_file(&self) -> bool {
        self.file_type.is_file()
    }
}

impl TryFrom<std::fs::Metadata> for Metadata {
    type Error = std::io::Error;

    fn try_from(metadata: std::fs::Metadata) -> Result<Self, Self::Error> {
        Ok(Self {
            file_type: metadata.file_type().try_into()?,
        })
    }
}
