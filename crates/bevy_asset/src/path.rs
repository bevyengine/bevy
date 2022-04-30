use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_utils::AHasher;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

#[derive(Debug, Hash, Clone, Serialize, Deserialize)]
pub struct AssetPath<'a> {
    path: Cow<'a, Path>,
    label: Option<Cow<'a, str>>,
}

impl<'a> AssetPath<'a> {
    #[inline]
    pub fn new_ref(path: &'a Path, label: Option<&'a str>) -> AssetPath<'a> {
        AssetPath {
            path: Cow::Borrowed(path),
            label: label.map(Cow::Borrowed),
        }
    }

    #[inline]
    pub fn new(path: PathBuf, label: Option<String>) -> AssetPath<'a> {
        AssetPath {
            path: Cow::Owned(path),
            label: label.map(Cow::Owned),
        }
    }

    #[inline]
    pub fn get_id(&self) -> AssetPathId {
        AssetPathId::from(self)
    }

    #[inline]
    pub fn label(&self) -> Option<&str> {
        self.label.as_ref().map(|label| label.as_ref())
    }

    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

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

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(PartialEq, Hash, Deserialize)]
pub struct AssetPathId(SourcePathId, LabelId);

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(PartialEq, Hash, Deserialize)]
pub struct SourcePathId(u64);

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect,
)]
#[reflect_value(PartialEq, Hash, Deserialize)]
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
    pub fn source_path_id(&self) -> SourcePathId {
        self.0
    }

    pub fn label_id(&self) -> LabelId {
        self.1
    }
}

/// this hasher provides consistent results across runs
pub(crate) fn get_hasher() -> AHasher {
    AHasher::new_with_keys(42, 23)
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
        let mut parts = asset_path.split('#');
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
