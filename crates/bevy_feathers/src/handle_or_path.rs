//! Provides a way to specify assets either by handle or by path.
use bevy_asset::{Asset, Handle};
use bevy_reflect::Reflect;

/// Enum that represents a reference to an asset as either a [`Handle`] or a [`String`] path.
///
/// This is useful for when you want to specify an asset, but don't always have convenient
/// access to an asset server reference.
#[derive(Clone, Debug, Reflect)]
pub enum HandleOrPath<T: Asset> {
    /// Specify the asset reference as a handle.
    Handle(Handle<T>),
    /// Specify the asset reference as a [`String`].
    Path(String),
}

impl<T: Asset> Default for HandleOrPath<T> {
    fn default() -> Self {
        Self::Path("".to_string())
    }
}

// Necessary because we don't want to require T: PartialEq
impl<T: Asset> PartialEq for HandleOrPath<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HandleOrPath::Handle(h1), HandleOrPath::Handle(h2)) => h1 == h2,
            (HandleOrPath::Path(p1), HandleOrPath::Path(p2)) => p1 == p2,
            _ => false,
        }
    }
}

impl<T: Asset> From<Handle<T>> for HandleOrPath<T> {
    fn from(h: Handle<T>) -> Self {
        HandleOrPath::Handle(h)
    }
}

impl<T: Asset> From<&str> for HandleOrPath<T> {
    fn from(p: &str) -> Self {
        HandleOrPath::Path(p.to_string())
    }
}

impl<T: Asset> From<String> for HandleOrPath<T> {
    fn from(p: String) -> Self {
        HandleOrPath::Path(p.clone())
    }
}

impl<T: Asset> From<&String> for HandleOrPath<T> {
    fn from(p: &String) -> Self {
        HandleOrPath::Path(p.to_string())
    }
}

impl<T: Asset + Clone> From<&HandleOrPath<T>> for HandleOrPath<T> {
    fn from(p: &HandleOrPath<T>) -> Self {
        p.to_owned()
    }
}
