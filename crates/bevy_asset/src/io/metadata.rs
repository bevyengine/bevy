/// A enum representing a type of file.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    Directory,
    File,
    // To be compatible with [`std::fs::FileType`].
    Symlink,
}

impl FileType {
    pub const fn is_dir(&self) -> bool {
        (*self as isize) == (Self::Directory as isize)
    }

    pub const fn is_file(&self) -> bool {
        (*self as isize) == (Self::File as isize)
    }
}

impl From<std::fs::FileType> for FileType {
    fn from(file_type: std::fs::FileType) -> Self {
        if file_type.is_dir() {
            Self::Directory
        } else if file_type.is_file() {
            Self::File
        } else if file_type.is_symlink() {
            Self::Symlink
        } else {
            unreachable!()
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

impl From<std::fs::Metadata> for Metadata {
    fn from(metadata: std::fs::Metadata) -> Self {
        Self {
            file_type: metadata.file_type().into(),
        }
    }
}
