use std::convert::{TryFrom, TryInto};

/// A enum representing a type of file.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    /// A directory.
    Directory,
    /// A file.
    File,
}

impl FileType {
    /// Returns `true` if the entry is a directory.
    #[inline]
    pub const fn is_dir(&self) -> bool {
        matches!(self, Self::Directory)
    }

    #[inline]
    /// Returns `true` if the entry is a file.
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File)
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
/// This structure is returned from the [`AssetIo::get_metadata`](crate::AssetIo) method.
#[derive(Debug, Clone)]
pub struct Metadata {
    file_type: FileType,
}

impl Metadata {
    /// Creates new metadata information.
    pub fn new(file_type: FileType) -> Self {
        Self { file_type }
    }

    /// Returns the file type.
    #[inline]
    pub const fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Returns `true` if the entry is a directory.
    #[inline]
    pub const fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    /// Returns `true` if the entry is a file.
    #[inline]
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
