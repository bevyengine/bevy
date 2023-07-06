use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_utils::{AHasher, RandomState};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    hash::{BuildHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

/// Represents a path to an asset in the file system.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Debug, PartialEq, Hash, Serialize, Deserialize)]
pub struct AssetPath<'a> {
    path: Cow<'a, Path>,
    label: Option<Cow<'a, str>>,
}

impl<'a> AssetPath<'a> {
    /// Creates a new asset path using borrowed information.
    #[inline]
    pub fn new_ref(path: &'a Path, label: Option<&'a str>) -> AssetPath<'a> {
        AssetPath {
            path: Cow::Borrowed(path),
            label: label.map(Cow::Borrowed),
        }
    }

    /// Creates a new asset path.
    #[inline]
    pub fn new(path: PathBuf, label: Option<String>) -> AssetPath<'a> {
        AssetPath {
            path: Cow::Owned(path),
            label: label.map(Cow::Owned),
        }
    }

    /// Constructs an identifier from this asset path.
    #[inline]
    pub fn get_id(&self) -> AssetPathId {
        AssetPathId::from(self)
    }

    /// Gets the sub-asset label.
    #[inline]
    pub fn label(&self) -> Option<&str> {
        self.label.as_ref().map(|label| label.as_ref())
    }

    /// Gets the path to the asset in the filesystem.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Converts the borrowed path data to owned.
    #[inline]
    pub fn to_owned(&self) -> AssetPath<'static> {
        AssetPath {
            path: Cow::Owned(self.path.to_path_buf()),
            label: self
                .label
                .as_ref()
                .map(|value| Cow::Owned(value.to_string())),
        }
    }
}

/// An unique identifier to an asset path.
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(PartialEq, Hash, Serialize, Deserialize)]
pub struct AssetPathId(SourcePathId, LabelId);

/// An unique identifier to the source path of an asset.
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(PartialEq, Hash, Serialize, Deserialize)]
pub struct SourcePathId(u64);

/// An unique identifier to a sub-asset label.
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(PartialEq, Hash, Serialize, Deserialize)]
pub struct LabelId(u64);

impl<'a> From<&'a Path> for SourcePathId {
    fn from(value: &'a Path) -> Self {
        let mut hasher = get_hasher();
        value.hash(&mut hasher);
        SourcePathId(hasher.finish())
    }
}

impl From<AssetPathId> for SourcePathId {
    fn from(id: AssetPathId) -> Self {
        id.source_path_id()
    }
}

impl<'a> From<AssetPath<'a>> for SourcePathId {
    fn from(path: AssetPath) -> Self {
        AssetPathId::from(path).source_path_id()
    }
}

impl<'a> From<Option<&'a str>> for LabelId {
    fn from(value: Option<&'a str>) -> Self {
        let mut hasher = get_hasher();
        value.hash(&mut hasher);
        LabelId(hasher.finish())
    }
}

impl AssetPathId {
    /// Gets the id of the source path.
    pub fn source_path_id(&self) -> SourcePathId {
        self.0
    }

    /// Gets the id of the sub-asset label.
    pub fn label_id(&self) -> LabelId {
        self.1
    }
}

/// this hasher provides consistent results across runs
pub(crate) fn get_hasher() -> AHasher {
    RandomState::with_seeds(42, 23, 13, 8).build_hasher()
}

impl<'a, T> From<T> for AssetPathId
where
    T: Into<AssetPath<'a>>,
{
    fn from(value: T) -> Self {
        let asset_path: AssetPath = value.into();
        AssetPathId(
            SourcePathId::from(asset_path.path()),
            LabelId::from(asset_path.label()),
        )
    }
}

impl<'a, 'b> From<&'a AssetPath<'b>> for AssetPathId {
    fn from(asset_path: &'a AssetPath<'b>) -> Self {
        AssetPathId(
            SourcePathId::from(asset_path.path()),
            LabelId::from(asset_path.label()),
        )
    }
}

impl<'a> From<&'a str> for AssetPath<'a> {
    fn from(asset_path: &'a str) -> Self {
        let mut parts = asset_path.splitn(2, '#');
        let path = Path::new(parts.next().expect("Path must be set."));
        let label = parts.next();
        AssetPath {
            path: Cow::Borrowed(path),
            label: label.map(Cow::Borrowed),
        }
    }
}

impl<'a> From<&'a String> for AssetPath<'a> {
    fn from(asset_path: &'a String) -> Self {
        asset_path.as_str().into()
    }
}

impl<'a> From<&'a Path> for AssetPath<'a> {
    fn from(path: &'a Path) -> Self {
        AssetPath {
            path: Cow::Borrowed(path),
            label: None,
        }
    }
}

impl<'a> From<PathBuf> for AssetPath<'a> {
    fn from(path: PathBuf) -> Self {
        AssetPath {
            path: Cow::Owned(path),
            label: None,
        }
    }
}

impl<'a> From<String> for AssetPath<'a> {
    fn from(asset_path: String) -> Self {
        let mut parts = asset_path.splitn(2, '#');
        let path = PathBuf::from(parts.next().expect("Path must be set."));
        let label = parts.next().map(String::from);
        AssetPath {
            path: Cow::Owned(path),
            label: label.map(Cow::Owned),
        }
    }
}
