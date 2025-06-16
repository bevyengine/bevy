use bevy_asset::{Asset, Handle};

/// Enum that represents either a [`Handle`] or a [`String`].
///
/// This is useful for when you want to specify an asset, but also want it to be have lifetime 'static
#[derive(Clone, Debug)]
pub enum HandleOrOwnedPath<T: Asset> {
    /// Specify the asset reference as a handle.
    Handle(Handle<T>),
    /// Specify the asset reference as a [`String`].
    Path(String),
}

impl<T: Asset> Default for HandleOrOwnedPath<T> {
    fn default() -> Self {
        Self::Path("".to_string())
    }
}

// Necessary because we don't want to require T: PartialEq
impl<T: Asset> PartialEq for HandleOrOwnedPath<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HandleOrOwnedPath::Handle(h1), HandleOrOwnedPath::Handle(h2)) => h1 == h2,
            (HandleOrOwnedPath::Path(p1), HandleOrOwnedPath::Path(p2)) => p1 == p2,
            _ => false,
        }
    }
}

impl<T: Asset> From<Handle<T>> for HandleOrOwnedPath<T> {
    fn from(h: Handle<T>) -> Self {
        HandleOrOwnedPath::Handle(h)
    }
}

impl<T: Asset> From<&str> for HandleOrOwnedPath<T> {
    fn from(p: &str) -> Self {
        HandleOrOwnedPath::Path(p.to_string())
    }
}

impl<T: Asset> From<String> for HandleOrOwnedPath<T> {
    fn from(p: String) -> Self {
        HandleOrOwnedPath::Path(p.clone())
    }
}

impl<T: Asset> From<&String> for HandleOrOwnedPath<T> {
    fn from(p: &String) -> Self {
        HandleOrOwnedPath::Path(p.to_string())
    }
}

impl<T: Asset + Clone> From<&HandleOrOwnedPath<T>> for HandleOrOwnedPath<T> {
    fn from(p: &HandleOrOwnedPath<T>) -> Self {
        p.to_owned()
    }
}
