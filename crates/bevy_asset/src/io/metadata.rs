/// A enum representing a type of file.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileType {
    Directory,
    File,
    // To be compatible with [`std::fs::FileType`].
    Symlink,
}

impl FileType {
    pub fn is_directory(&self) -> bool {
        *self == Self::Directory
    }

    pub fn is_file(&self) -> bool {
        *self == Self::File
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
#[derive(Debug)]
pub struct Metadata {
    pub file_type: FileType,
}

impl Metadata {
    pub fn is_directory(&self) -> bool {
        self.file_type.is_directory()
    }

    pub fn is_file(&self) -> bool {
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
