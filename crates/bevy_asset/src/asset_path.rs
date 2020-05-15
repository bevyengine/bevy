use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct AssetPath {
    pub path: Cow<'static, str>,
    pub extension: Option<Cow<'static, str>>,
}

impl From<&Path> for AssetPath {
    fn from(path: &Path) -> Self {
        AssetPath {
            path: Cow::Owned(
                path.to_str()
                    .expect("Path should be a valid string.")
                    .to_string(),
            ),
            extension: path.extension().map(|e| {
                Cow::Owned(
                    e.to_str()
                        .expect("Extension should be a valid string.")
                        .to_string(),
                )
            }),
        }
    }
}

impl From<&PathBuf> for AssetPath {
    fn from(path: &PathBuf) -> Self {
        AssetPath {
            path: Cow::Owned(
                path.to_str()
                    .expect("Path should be a valid string.")
                    .to_string(),
            ),
            extension: path.extension().map(|e| {
                Cow::Owned(
                    e.to_str()
                        .expect("Extension should be a valid string.")
                        .to_string(),
                )
            }),
        }
    }
}

impl From<&str> for AssetPath {
    fn from(path: &str) -> Self {
        let path = Path::new(path);
        AssetPath::from(path)
    }
}
