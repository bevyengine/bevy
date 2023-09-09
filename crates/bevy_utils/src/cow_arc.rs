use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Much like a [`Cow`](std::borrow::Cow), but owned values are Arc-ed to make clones cheap. This should be used for values that
/// are cloned for use across threads and change rarely (if ever).
pub enum CowArc<'a, T: ?Sized + 'static> {
    /// A borrowed value
    Borrowed(&'a T),
    /// A static value reference
    Static(&'static T),
    /// An owned [`Arc`]-ed value
    Owned(Arc<T>),
}

impl<'a, T: ?Sized> Deref for CowArc<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            CowArc::Borrowed(v) => v,
            CowArc::Static(v) => v,
            CowArc::Owned(v) => v,
        }
    }
}

impl<'a, T: ?Sized> CowArc<'a, T>
where
    &'a T: Into<Arc<T>>,
{
    /// Converts this into an "owned" value. If internally a value is borrowed, it will be cloned into an "owned [`Arc`]".
    /// If it is already an "owned [`Arc`]", it will remain unchanged.
    #[inline]
    pub fn into_owned(self) -> CowArc<'static, T> {
        match self {
            CowArc::Borrowed(value) => CowArc::Owned(value.into()),
            CowArc::Static(value) => CowArc::Static(value),
            CowArc::Owned(value) => CowArc::Owned(value),
        }
    }
}

impl<'a, T: ?Sized> Clone for CowArc<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(value) => Self::Borrowed(value),
            Self::Static(value) => Self::Static(value),
            Self::Owned(value) => Self::Owned(value.clone()),
        }
    }
}

impl<'a, T: PartialEq + ?Sized> PartialEq for CowArc<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<'a, T: PartialEq + ?Sized> Eq for CowArc<'a, T> {}

impl<'a, T: Hash + ?Sized> Hash for CowArc<'a, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl<'a, T: Debug + ?Sized> Debug for CowArc<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}

impl<'a, T: Display + ?Sized> Display for CowArc<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.deref(), f)
    }
}

impl<'a, T: PartialOrd + ?Sized> PartialOrd for CowArc<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl<'a, T: Ord + ?Sized> Ord for CowArc<'a, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deref().cmp(other.deref())
    }
}

impl From<PathBuf> for CowArc<'static, Path> {
    #[inline]
    fn from(value: PathBuf) -> Self {
        CowArc::Owned(value.into())
    }
}

impl From<&'static str> for CowArc<'static, Path> {
    #[inline]
    fn from(value: &'static str) -> Self {
        CowArc::Static(Path::new(value))
    }
}

impl From<String> for CowArc<'static, str> {
    #[inline]
    fn from(value: String) -> Self {
        CowArc::Owned(value.into())
    }
}

impl<'a> From<&'a String> for CowArc<'a, str> {
    #[inline]
    fn from(value: &'a String) -> Self {
        CowArc::Borrowed(value)
    }
}

impl<T: ?Sized> From<&'static T> for CowArc<'static, T> {
    #[inline]
    fn from(value: &'static T) -> Self {
        CowArc::Static(value)
    }
}
