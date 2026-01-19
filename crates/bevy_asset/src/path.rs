use crate::io::AssetSourceId;
use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec::Vec,
};
use atomicow::CowArc;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use core::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::Deref,
};
use serde::{de::Visitor, Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Represents a path to an asset in a "virtual filesystem".
///
/// Asset paths consist of three main parts:
/// * [`AssetPath::source`]: The name of the [`AssetSource`](crate::io::AssetSource) to load the asset from.
///   This is optional. If one is not set the default source will be used (which is the `assets` folder by default).
/// * [`AssetPath::path`]: The "virtual filesystem path" pointing to an asset source file.
/// * [`AssetPath::label`]: An optional "named sub asset". When assets are loaded, they are
///   allowed to load "sub assets" of any type, which are identified by a named "label".
///
/// Asset paths are generally constructed (and visualized) as strings:
///
/// ```no_run
/// # use bevy_asset::{Asset, AssetServer, Handle};
/// # use bevy_reflect::TypePath;
/// #
/// # #[derive(Asset, TypePath, Default)]
/// # struct Mesh;
/// #
/// # #[derive(Asset, TypePath, Default)]
/// # struct Scene;
/// #
/// # let asset_server: AssetServer = panic!();
/// // This loads the `my_scene.scn` base asset from the default asset source.
/// let scene: Handle<Scene> = asset_server.load("my_scene.scn");
///
/// // This loads the `PlayerMesh` labeled asset from the `my_scene.scn` base asset in the default asset source.
/// let mesh: Handle<Mesh> = asset_server.load("my_scene.scn#PlayerMesh");
///
/// // This loads the `my_scene.scn` base asset from a custom 'remote' asset source.
/// let scene: Handle<Scene> = asset_server.load("remote://my_scene.scn");
/// ```
///
/// [`AssetPath`] implements [`From`] for `&'static str`, `&'static Path`, and `&'a String`,
/// which allows us to optimize the static cases.
/// This means that the common case of `asset_server.load("my_scene.scn")` when it creates and
/// clones internal owned [`AssetPaths`](AssetPath).
/// This also means that you should use [`AssetPath::parse`] in cases where `&str` is the explicit type.
#[derive(Eq, PartialEq, Hash, Clone, Default, Reflect)]
#[reflect(opaque)]
#[reflect(Debug, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct AssetPath<'a> {
    source: AssetSourceId<'a>,
    path: CowArc<'a, Path>,
    label: Option<CowArc<'a, str>>,
}

impl<'a> Debug for AssetPath<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<'a> Display for AssetPath<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let AssetSourceId::Name(name) = self.source() {
            write!(f, "{name}://")?;
        }
        write!(f, "{}", self.path.display())?;
        if let Some(label) = &self.label {
            write!(f, "#{label}")?;
        }
        Ok(())
    }
}

/// An error that occurs when parsing a string type to create an [`AssetPath`] fails, such as during [`AssetPath::parse`].
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParseAssetPathError {
    /// Error that occurs when the [`AssetPath::source`] section of a path string contains the [`AssetPath::label`] delimiter `#`. E.g. `bad#source://file.test`.
    #[error("Asset source must not contain a `#` character")]
    InvalidSourceSyntax,
    /// Error that occurs when the [`AssetPath::label`] section of a path string contains the [`AssetPath::source`] delimiter `://`. E.g. `source://file.test#bad://label`.
    #[error("Asset label must not contain a `://` substring")]
    InvalidLabelSyntax,
    /// Error that occurs when a path string has an [`AssetPath::source`] delimiter `://` with no characters preceding it. E.g. `://file.test`.
    #[error("Asset source must be at least one character. Either specify the source before the '://' or remove the `://`")]
    MissingSource,
    /// Error that occurs when a path string has an [`AssetPath::label`] delimiter `#` with no characters succeeding it. E.g. `file.test#`
    #[error("Asset label must be at least one character. Either specify the label after the '#' or remove the '#'")]
    MissingLabel,
}

impl<'a> AssetPath<'a> {
    /// Creates a new [`AssetPath`] from a string in the asset path format:
    /// * An asset at the root: `"scene.gltf"`
    /// * An asset nested in some folders: `"some/path/scene.gltf"`
    /// * An asset with a "label": `"some/path/scene.gltf#Mesh0"`
    /// * An asset with a custom "source": `"custom://some/path/scene.gltf#Mesh0"`
    ///
    /// Prefer [`From<'static str>`] for static strings, as this will prevent allocations
    /// and reference counting for [`AssetPath::into_owned`].
    ///
    /// # Panics
    /// Panics if the asset path is in an invalid format. Use [`AssetPath::try_parse`] for a fallible variant
    pub fn parse(asset_path: &'a str) -> AssetPath<'a> {
        Self::try_parse(asset_path).unwrap()
    }

    /// Creates a new [`AssetPath`] from a string in the asset path format:
    /// * An asset at the root: `"scene.gltf"`
    /// * An asset nested in some folders: `"some/path/scene.gltf"`
    /// * An asset with a "label": `"some/path/scene.gltf#Mesh0"`
    /// * An asset with a custom "source": `"custom://some/path/scene.gltf#Mesh0"`
    ///
    /// Prefer [`From<'static str>`] for static strings, as this will prevent allocations
    /// and reference counting for [`AssetPath::into_owned`].
    ///
    /// This will return a [`ParseAssetPathError`] if `asset_path` is in an invalid format.
    pub fn try_parse(asset_path: &'a str) -> Result<AssetPath<'a>, ParseAssetPathError> {
        let (source, path, label) = Self::parse_internal(asset_path)?;
        Ok(Self {
            source: match source {
                Some(source) => AssetSourceId::Name(CowArc::Borrowed(source)),
                None => AssetSourceId::Default,
            },
            path: CowArc::Borrowed(path),
            label: label.map(CowArc::Borrowed),
        })
    }

    // Attempts to Parse a &str into an `AssetPath`'s `AssetPath::source`, `AssetPath::path`, and `AssetPath::label` components.
    fn parse_internal(
        asset_path: &str,
    ) -> Result<(Option<&str>, &Path, Option<&str>), ParseAssetPathError> {
        let chars = asset_path.char_indices();
        let mut source_range = None;
        let mut path_range = 0..asset_path.len();
        let mut label_range = None;

        // Loop through the characters of the passed in &str to accomplish the following:
        // 1. Search for the first instance of the `://` substring. If the `://` substring is found,
        //  store the range of indices representing everything before the `://` substring as the `source_range`.
        // 2. Search for the last instance of the `#` character. If the `#` character is found,
        //  store the range of indices representing everything after the `#` character as the `label_range`
        // 3. Set the `path_range` to be everything in between the `source_range` and `label_range`,
        //  excluding the `://` substring and `#` character.
        // 4. Verify that there are no `#` characters in the `AssetPath::source` and no `://` substrings in the `AssetPath::label`
        let mut source_delimiter_chars_matched = 0;
        let mut last_found_source_index = 0;
        for (index, char) in chars {
            match char {
                ':' => {
                    source_delimiter_chars_matched = 1;
                }
                '/' => {
                    match source_delimiter_chars_matched {
                        1 => {
                            source_delimiter_chars_matched = 2;
                        }
                        2 => {
                            // If we haven't found our first `AssetPath::source` yet, check to make sure it is valid and then store it.
                            if source_range.is_none() {
                                // If the `AssetPath::source` contains a `#` character, it is invalid.
                                if label_range.is_some() {
                                    return Err(ParseAssetPathError::InvalidSourceSyntax);
                                }
                                source_range = Some(0..index - 2);
                                path_range.start = index + 1;
                            }
                            last_found_source_index = index - 2;
                            source_delimiter_chars_matched = 0;
                        }
                        _ => {}
                    }
                }
                '#' => {
                    path_range.end = index;
                    label_range = Some(index + 1..asset_path.len());
                    source_delimiter_chars_matched = 0;
                }
                _ => {
                    source_delimiter_chars_matched = 0;
                }
            }
        }
        // If we found an `AssetPath::label`
        if let Some(range) = label_range.clone() {
            // If the `AssetPath::label` contained a `://` substring, it is invalid.
            if range.start <= last_found_source_index {
                return Err(ParseAssetPathError::InvalidLabelSyntax);
            }
        }
        // Try to parse the range of indices that represents the `AssetPath::source` portion of the `AssetPath` to make sure it is not empty.
        // This would be the case if the input &str was something like `://some/file.test`
        let source = match source_range {
            Some(source_range) => {
                if source_range.is_empty() {
                    return Err(ParseAssetPathError::MissingSource);
                }
                Some(&asset_path[source_range])
            }
            None => None,
        };
        // Try to parse the range of indices that represents the `AssetPath::label` portion of the `AssetPath` to make sure it is not empty.
        // This would be the case if the input &str was something like `some/file.test#`.
        let label = match label_range {
            Some(label_range) => {
                if label_range.is_empty() {
                    return Err(ParseAssetPathError::MissingLabel);
                }
                Some(&asset_path[label_range])
            }
            None => None,
        };

        let path = Path::new(&asset_path[path_range]);
        Ok((source, path, label))
    }

    /// Creates a new [`AssetPath`] from a [`PathBuf`].
    #[inline]
    pub fn from_path_buf(path_buf: PathBuf) -> AssetPath<'a> {
        AssetPath {
            path: CowArc::Owned(path_buf.into()),
            source: AssetSourceId::Default,
            label: None,
        }
    }

    /// Creates a new [`AssetPath`] from a [`Path`].
    #[inline]
    pub fn from_path(path: &'a Path) -> AssetPath<'a> {
        AssetPath {
            path: CowArc::Borrowed(path),
            source: AssetSourceId::Default,
            label: None,
        }
    }

    /// Gets the "asset source", if one was defined. If none was defined, the default source
    /// will be used.
    #[inline]
    pub fn source(&self) -> &AssetSourceId<'_> {
        &self.source
    }

    /// Gets the "sub-asset label".
    #[inline]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Gets the "sub-asset label".
    #[inline]
    pub fn label_cow(&self) -> Option<CowArc<'a, str>> {
        self.label.clone()
    }

    /// Gets the path to the asset in the "virtual filesystem".
    #[inline]
    pub fn path(&self) -> &Path {
        self.path.deref()
    }

    /// Gets the path to the asset in the "virtual filesystem" without a label (if a label is currently set).
    #[inline]
    pub fn without_label(&self) -> AssetPath<'_> {
        Self {
            source: self.source.clone(),
            path: self.path.clone(),
            label: None,
        }
    }

    /// Removes a "sub-asset label" from this [`AssetPath`], if one was set.
    #[inline]
    pub fn remove_label(&mut self) {
        self.label = None;
    }

    /// Takes the "sub-asset label" from this [`AssetPath`], if one was set.
    #[inline]
    pub fn take_label(&mut self) -> Option<CowArc<'a, str>> {
        self.label.take()
    }

    /// Returns this asset path with the given label. This will replace the previous
    /// label if it exists.
    #[inline]
    pub fn with_label(self, label: impl Into<CowArc<'a, str>>) -> AssetPath<'a> {
        AssetPath {
            source: self.source,
            path: self.path,
            label: Some(label.into()),
        }
    }

    /// Returns this asset path with the given asset source. This will replace the previous asset
    /// source if it exists.
    #[inline]
    pub fn with_source(self, source: impl Into<AssetSourceId<'a>>) -> AssetPath<'a> {
        AssetPath {
            source: source.into(),
            path: self.path,
            label: self.label,
        }
    }

    /// Returns an [`AssetPath`] for the parent folder of this path, if there is a parent folder in the path.
    pub fn parent(&self) -> Option<AssetPath<'a>> {
        let path = match &self.path {
            CowArc::Borrowed(path) => CowArc::Borrowed(path.parent()?),
            CowArc::Static(path) => CowArc::Static(path.parent()?),
            CowArc::Owned(path) => path.parent()?.to_path_buf().into(),
        };
        Some(AssetPath {
            source: self.source.clone(),
            label: None,
            path,
        })
    }

    /// Converts this into an "owned" value. If internally a value is borrowed, it will be cloned into an "owned [`Arc`]".
    /// If internally a value is a static reference, the static reference will be used unchanged.
    /// If internally a value is an "owned [`Arc`]", it will remain unchanged.
    ///
    /// [`Arc`]: alloc::sync::Arc
    pub fn into_owned(self) -> AssetPath<'static> {
        AssetPath {
            source: self.source.into_owned(),
            path: self.path.into_owned(),
            label: self.label.map(CowArc::into_owned),
        }
    }

    /// Clones this into an "owned" value. If internally a value is borrowed, it will be cloned into an "owned [`Arc`]".
    /// If internally a value is a static reference, the static reference will be used unchanged.
    /// If internally a value is an "owned [`Arc`]", the [`Arc`] will be cloned.
    ///
    /// [`Arc`]: alloc::sync::Arc
    #[inline]
    pub fn clone_owned(&self) -> AssetPath<'static> {
        self.clone().into_owned()
    }

    /// Resolves an [`AssetPath`] relative to `self`.
    ///
    /// Semantics:
    /// - If `path` is label-only (default source, empty path, label set), replace `self`'s label.
    /// - If `path` begins with `/`, treat it as rooted at the asset-source root (not the filesystem).
    /// - If `path` has an explicit source (`name://...`), it replaces the base source.
    /// - Relative segments are concatenated and normalized (`.`/`..` removal), preserving extra `..` if the base underflows.
    ///
    /// ```
    /// # use bevy_asset::AssetPath;
    /// let base = AssetPath::parse("a/b");
    /// assert_eq!(base.resolve(&AssetPath::parse("c")), AssetPath::parse("a/b/c"));
    /// assert_eq!(base.resolve(&AssetPath::parse("./c")), AssetPath::parse("a/b/c"));
    /// assert_eq!(base.resolve(&AssetPath::parse("../c")), AssetPath::parse("a/c"));
    /// assert_eq!(base.resolve(&AssetPath::parse("c.png")), AssetPath::parse("a/b/c.png"));
    /// assert_eq!(base.resolve(&AssetPath::parse("/c")), AssetPath::parse("c"));
    /// assert_eq!(AssetPath::parse("a/b.png").resolve(&AssetPath::parse("#c")), AssetPath::parse("a/b.png#c"));
    /// assert_eq!(AssetPath::parse("a/b.png#c").resolve(&AssetPath::parse("#d")), AssetPath::parse("a/b.png#d"));
    /// ```
    ///
    /// See also [`AssetPath::resolve_str`].
    pub fn resolve(&self, path: &AssetPath<'_>) -> AssetPath<'static> {
        let is_label_only = matches!(path.source(), AssetSourceId::Default)
            && path.path().as_os_str().is_empty()
            && path.label().is_some();

        if is_label_only {
            self.clone_owned()
                .with_label(path.label().unwrap().to_owned())
        } else {
            let explicit_source = match path.source() {
                AssetSourceId::Default => None,
                AssetSourceId::Name(name) => Some(name.as_ref()),
            };

            self.resolve_from_parts(false, explicit_source, path.path(), path.label())
        }
    }

    /// Resolves an [`AssetPath`] relative to `self` using embedded (RFC 1808) semantics.
    ///
    /// Semantics:
    /// - Remove the "file portion" of the base before concatenation (unless the base ends with `/`).
    /// - Otherwise identical to [`AssetPath::resolve`].
    ///
    /// ```
    /// # use bevy_asset::AssetPath;
    /// let base = AssetPath::parse("a/b");
    /// assert_eq!(base.resolve_embed(&AssetPath::parse("c")), AssetPath::parse("a/c"));
    /// assert_eq!(base.resolve_embed(&AssetPath::parse("./c")), AssetPath::parse("a/c"));
    /// assert_eq!(base.resolve_embed(&AssetPath::parse("../c")), AssetPath::parse("c"));
    /// assert_eq!(base.resolve_embed(&AssetPath::parse("c.png")), AssetPath::parse("a/c.png"));
    /// assert_eq!(base.resolve_embed(&AssetPath::parse("/c")), AssetPath::parse("c"));
    /// assert_eq!(AssetPath::parse("a/b.png").resolve_embed(&AssetPath::parse("#c")), AssetPath::parse("a/b.png#c"));
    /// assert_eq!(AssetPath::parse("a/b.png#c").resolve_embed(&AssetPath::parse("#d")), AssetPath::parse("a/b.png#d"));
    /// ```
    ///
    /// See also [`AssetPath::resolve_embed_str`].
    pub fn resolve_embed(&self, path: &AssetPath<'_>) -> AssetPath<'static> {
        let is_label_only = matches!(path.source(), AssetSourceId::Default)
            && path.path().as_os_str().is_empty()
            && path.label().is_some();

        if is_label_only {
            self.clone_owned()
                .with_label(path.label().unwrap().to_owned())
        } else {
            let explicit_source = match path.source() {
                AssetSourceId::Default => None,
                AssetSourceId::Name(name) => Some(name.as_ref()),
            };

            self.resolve_from_parts(true, explicit_source, path.path(), path.label())
        }
    }

    /// Parses `path` as an [`AssetPath`], then resolves it relative to `self`.
    ///
    /// Returns an error if parsing fails.
    ///
    /// For more details, see [`AssetPath::resolve`].
    pub fn resolve_str(&self, path: &str) -> Result<AssetPath<'static>, ParseAssetPathError> {
        self.resolve_internal(path, false)
    }

    /// Parses `path` as an [`AssetPath`], then resolves it relative to `self` using embedded
    /// (RFC 1808) semantics.
    ///
    /// Returns an error if parsing fails.
    ///
    /// For more details, see [`AssetPath::resolve_embed`].
    pub fn resolve_embed_str(&self, path: &str) -> Result<AssetPath<'static>, ParseAssetPathError> {
        self.resolve_internal(path, true)
    }

    fn resolve_from_parts(
        &self,
        replace: bool,
        source: Option<&str>,
        rpath: &Path,
        rlabel: Option<&str>,
    ) -> AssetPath<'static> {
        let base_str = self
            .path()
            .to_str()
            .expect("asset path must be valid UTF-8");
        let base_trailing_slash = base_str.ends_with('/');
        let rpath_str = rpath.to_str().expect("asset path must be valid UTF-8");
        let rpath_is_rooted = rpath_str.starts_with('/');
        let rpath_str = if rpath_is_rooted {
            rpath_str.strip_prefix('/').unwrap_or(rpath_str)
        } else {
            rpath_str
        };

        let resolved = if source.is_some() {
            join_and_normalize_asset_path("", false, rpath_str, true, replace)
        } else {
            join_and_normalize_asset_path(
                base_str,
                base_trailing_slash,
                rpath_str,
                rpath_is_rooted,
                replace,
            )
        };

        AssetPath {
            source: match source {
                Some(source) => AssetSourceId::Name(CowArc::Owned(source.into())),
                None => self.source.clone_owned(),
            },
            path: CowArc::Owned(PathBuf::from(resolved).into()),
            label: rlabel.map(|l| CowArc::Owned(l.into())),
        }
    }

    fn resolve_internal(
        &self,
        path: &str,
        replace: bool,
    ) -> Result<AssetPath<'static>, ParseAssetPathError> {
        if let Some(label) = path.strip_prefix('#') {
            // It's a label only
            Ok(self.clone_owned().with_label(label.to_owned()))
        } else {
            let (source, rpath, rlabel) = AssetPath::parse_internal(path)?;
            Ok(self.resolve_from_parts(replace, source, rpath, rlabel))
        }
    }

    /// Returns the full extension (including multiple '.' values).
    /// Ex: Returns `"config.ron"` for `"my_asset.config.ron"`
    ///
    /// Also strips out anything following a `?` to handle query parameters in URIs
    pub fn get_full_extension(&self) -> Option<String> {
        let file_name = self.path().file_name()?.to_str()?;
        let index = file_name.find('.')?;
        let mut extension = file_name[index + 1..].to_owned();

        // Strip off any query parameters
        let query = extension.find('?');
        if let Some(offset) = query {
            extension.truncate(offset);
        }

        Some(extension)
    }

    pub(crate) fn iter_secondary_extensions(full_extension: &str) -> impl Iterator<Item = &str> {
        full_extension.char_indices().filter_map(|(i, c)| {
            if c == '.' {
                Some(&full_extension[i + 1..])
            } else {
                None
            }
        })
    }

    /// Returns `true` if this [`AssetPath`] points to a file that is
    /// outside of its [`AssetSource`](crate::io::AssetSource) folder.
    ///
    /// ## Example
    /// ```
    /// # use bevy_asset::AssetPath;
    /// // Inside the default AssetSource.
    /// let path = AssetPath::parse("thingy.png");
    /// assert!( ! path.is_unapproved());
    /// let path = AssetPath::parse("gui/thingy.png");
    /// assert!( ! path.is_unapproved());
    ///
    /// // Inside a different AssetSource.
    /// let path = AssetPath::parse("embedded://thingy.png");
    /// assert!( ! path.is_unapproved());
    ///
    /// // Exits the `AssetSource`s directory.
    /// let path = AssetPath::parse("../thingy.png");
    /// assert!(path.is_unapproved());
    /// let path = AssetPath::parse("folder/../../thingy.png");
    /// assert!(path.is_unapproved());
    ///
    /// // This references the linux root directory.
    /// let path = AssetPath::parse("/home/thingy.png");
    /// assert!(path.is_unapproved());
    /// ```
    pub fn is_unapproved(&self) -> bool {
        use std::path::Component;
        let mut simplified = PathBuf::new();
        for component in self.path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => return true,
                Component::CurDir => {}
                Component::ParentDir => {
                    if !simplified.pop() {
                        return true;
                    }
                }
                Component::Normal(os_str) => simplified.push(os_str),
            }
        }

        false
    }
}

// This is only implemented for static lifetimes to ensure `Path::clone` does not allocate
// by ensuring that this is stored as a `CowArc::Static`.
// Please read https://github.com/bevyengine/bevy/issues/19844 before changing this!
impl From<&'static str> for AssetPath<'static> {
    #[inline]
    fn from(asset_path: &'static str) -> Self {
        let (source, path, label) = Self::parse_internal(asset_path).unwrap();
        AssetPath {
            source: source.into(),
            path: CowArc::Static(path),
            label: label.map(CowArc::Static),
        }
    }
}

impl<'a> From<&'a String> for AssetPath<'a> {
    #[inline]
    fn from(asset_path: &'a String) -> Self {
        AssetPath::parse(asset_path.as_str())
    }
}

impl From<String> for AssetPath<'static> {
    #[inline]
    fn from(asset_path: String) -> Self {
        AssetPath::parse(asset_path.as_str()).into_owned()
    }
}

impl From<&'static Path> for AssetPath<'static> {
    #[inline]
    fn from(path: &'static Path) -> Self {
        Self {
            source: AssetSourceId::Default,
            path: CowArc::Static(path),
            label: None,
        }
    }
}

impl From<PathBuf> for AssetPath<'static> {
    #[inline]
    fn from(path: PathBuf) -> Self {
        Self {
            source: AssetSourceId::Default,
            path: path.into(),
            label: None,
        }
    }
}

impl<'a, 'b> From<&'a AssetPath<'b>> for AssetPath<'b> {
    fn from(value: &'a AssetPath<'b>) -> Self {
        value.clone()
    }
}

impl<'a> From<AssetPath<'a>> for PathBuf {
    fn from(value: AssetPath<'a>) -> Self {
        value.path().to_path_buf()
    }
}

impl<'a> Serialize for AssetPath<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AssetPath<'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(AssetPathVisitor)
    }
}

struct AssetPathVisitor;

impl<'de> Visitor<'de> for AssetPathVisitor {
    type Value = AssetPath<'static>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("string AssetPath")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(AssetPath::parse(v).into_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(AssetPath::from(v))
    }
}

/*
function that splits only on /, and returns (bool, Vec<&str>):
bool = path starts with /
Vec<&str> = segments between / (define once whether you keep or drop "" from // or trailing / and stick to it)
*/

pub fn split_asset_path_segments(path_str: &str) -> (bool, Vec<&str>) {
    let is_rooted = path_str.starts_with('/');
    let to_split = path_str.strip_prefix('/').unwrap_or(path_str);
    let segments: Vec<&str> = to_split.split('/').collect();
    (is_rooted, segments)
}

/// Normalizes segments by applying '.' and '..' rules
/// as per [RFC 1808](https://datatracker.ietf.org/doc/html/rfc1808)
/// 'is_rooted' is reserved for future use (e.g. forbidding '..' above root). current behavior matches 'normalize_path'.
pub(crate) fn normalize_asset_path_segments(segments: &[&str], _is_rooted: bool) -> Vec<String> {
    let mut result = Vec::new();
    for segment in segments {
        if *segment == "." {
            // Skip
        } else if *segment == ".." {
            // Pop the last segment if it exists and is not "..", otherwise preserve ".." (underflow).
            if !result.is_empty() && result.last().unwrap() != ".." {
                result.pop();
            } else {
                result.push("..".to_string());
            }
        } else {
            result.push(segment.to_string());
        }
    }
    result
}

/// Check normalize_asset_path_segments above for the implementation of this function.
#[allow(dead_code)]
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut result_path = PathBuf::new();
    for elt in path.iter() {
        if elt == "." {
            // Skip
        } else if elt == ".." {
            // Note: If the result_path ends in `..`, Path::file_name returns None, so we'll end up
            // preserving it.
            if result_path.file_name().is_some() {
                // This assert is just a sanity check - we already know the path has a file_name, so
                // we know there is something to pop.
                assert!(result_path.pop());
            } else {
                // Preserve ".." if insufficient matches (per RFC 1808).
                result_path.push(elt);
            }
        } else {
            result_path.push(elt);
        }
    }
    result_path
}

/// Joins 'base_str' and 'rpath_str' then normalizes. Used by 'resolve_from_parts'.
/// - 'rpath_is_rooted': if true, base is ignored and the result is rooted.
/// - 'replace': if true (resolve_embed), drop the last base segment unless 'base_trailing_slash'.
/// - 'rpath_str' must already have a leading '/' stripped when 'rpath_is_rooted' is true.
pub(crate) fn join_and_normalize_asset_path(
    base_str: &str,
    base_trailing_slash: bool,
    rpath_str: &str,
    rpath_is_rooted: bool,
    replace: bool,
) -> String {
    if rpath_is_rooted {
        let (_, rpath_segments) = split_asset_path_segments(rpath_str);
        let normalized = normalize_asset_path_segments(&rpath_segments, true);
        return normalized.join("/");
    }

    let (base_rooted, mut base_segments) = split_asset_path_segments(base_str);
    while base_segments.last() == Some(&"") {
        base_segments.pop();
    }
    let (_, rpath_segments) = split_asset_path_segments(rpath_str);

    let base_use: &[&str] = if replace && !base_trailing_slash && !base_segments.is_empty() {
        let n = base_segments.len() - 1;
        &base_segments[..n]
    } else {
        &base_segments[..]
    };

    let mut combined = Vec::new();
    combined.extend(base_use);
    combined.extend(rpath_segments.iter().copied());

    let normalized = normalize_asset_path_segments(&combined, base_rooted);
    let joined = normalized.join("/");

    if base_rooted {
        "/".to_string() + &joined
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::join_and_normalize_asset_path;
    use super::normalize_asset_path_segments;
    use super::split_asset_path_segments;
    use crate::AssetPath;
    use alloc::string::ToString;
    use alloc::vec;
    use std::path::Path;

    #[test]
    fn parse_asset_path() {
        let result = AssetPath::parse_internal("a/b.test");
        assert_eq!(result, Ok((None, Path::new("a/b.test"), None)));

        let result = AssetPath::parse_internal("http://a/b.test");
        assert_eq!(result, Ok((Some("http"), Path::new("a/b.test"), None)));

        let result = AssetPath::parse_internal("http://a/b.test#Foo");
        assert_eq!(
            result,
            Ok((Some("http"), Path::new("a/b.test"), Some("Foo")))
        );

        let result = AssetPath::parse_internal("localhost:80/b.test");
        assert_eq!(result, Ok((None, Path::new("localhost:80/b.test"), None)));

        let result = AssetPath::parse_internal("http://localhost:80/b.test");
        assert_eq!(
            result,
            Ok((Some("http"), Path::new("localhost:80/b.test"), None))
        );

        let result = AssetPath::parse_internal("http://localhost:80/b.test#Foo");
        assert_eq!(
            result,
            Ok((Some("http"), Path::new("localhost:80/b.test"), Some("Foo")))
        );

        let result = AssetPath::parse_internal("#insource://a/b.test");
        assert_eq!(result, Err(crate::ParseAssetPathError::InvalidSourceSyntax));

        let result = AssetPath::parse_internal("source://a/b.test#://inlabel");
        assert_eq!(result, Err(crate::ParseAssetPathError::InvalidLabelSyntax));

        let result = AssetPath::parse_internal("#insource://a/b.test#://inlabel");
        assert!(
            result == Err(crate::ParseAssetPathError::InvalidSourceSyntax)
                || result == Err(crate::ParseAssetPathError::InvalidLabelSyntax)
        );

        let result = AssetPath::parse_internal("http://");
        assert_eq!(result, Ok((Some("http"), Path::new(""), None)));

        let result = AssetPath::parse_internal("://x");
        assert_eq!(result, Err(crate::ParseAssetPathError::MissingSource));

        let result = AssetPath::parse_internal("a/b.test#");
        assert_eq!(result, Err(crate::ParseAssetPathError::MissingLabel));
    }

    #[test]
    fn test_parent() {
        // Parent consumes path segments, returns None when insufficient
        let result = AssetPath::from("a/b.test");
        assert_eq!(result.parent(), Some(AssetPath::from("a")));
        assert_eq!(result.parent().unwrap().parent(), Some(AssetPath::from("")));
        assert_eq!(result.parent().unwrap().parent().unwrap().parent(), None);

        // Parent cannot consume asset source
        let result = AssetPath::from("http://a");
        assert_eq!(result.parent(), Some(AssetPath::from("http://")));
        assert_eq!(result.parent().unwrap().parent(), None);

        // Parent consumes labels
        let result = AssetPath::from("http://a#Foo");
        assert_eq!(result.parent(), Some(AssetPath::from("http://")));
    }

    #[test]
    fn test_with_source() {
        let result = AssetPath::from("http://a#Foo");
        assert_eq!(result.with_source("ftp"), AssetPath::from("ftp://a#Foo"));
    }

    #[test]
    fn test_without_label() {
        let result = AssetPath::from("http://a#Foo");
        assert_eq!(result.without_label(), AssetPath::from("http://a"));
    }

    #[test]
    fn test_resolve_full() {
        // A "full" path should ignore the base path.
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("/joe/next").unwrap(),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/joe/next")),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve_embed_str("/joe/next").unwrap(),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/joe/next")),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve_str("/joe/next#dave").unwrap(),
            AssetPath::from("joe/next#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/joe/next#dave")),
            AssetPath::from("joe/next#dave")
        );
        assert_eq!(
            base.resolve_embed_str("/joe/next#dave").unwrap(),
            AssetPath::from("joe/next#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/joe/next#dave")),
            AssetPath::from("joe/next#dave")
        );
    }

    #[test]
    fn test_resolve_implicit_relative() {
        // A path with no initial directory separator should be considered relative.
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("joe/next").unwrap(),
            AssetPath::from("alice/bob/joe/next")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("joe/next")),
            AssetPath::from("alice/bob/joe/next")
        );
        assert_eq!(
            base.resolve_embed_str("joe/next").unwrap(),
            AssetPath::from("alice/joe/next")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("joe/next")),
            AssetPath::from("alice/joe/next")
        );
        assert_eq!(
            base.resolve_str("joe/next#dave").unwrap(),
            AssetPath::from("alice/bob/joe/next#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("joe/next#dave")),
            AssetPath::from("alice/bob/joe/next#dave")
        );
        assert_eq!(
            base.resolve_embed_str("joe/next#dave").unwrap(),
            AssetPath::from("alice/joe/next#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("joe/next#dave")),
            AssetPath::from("alice/joe/next#dave")
        );
    }

    #[test]
    fn test_resolve_explicit_relative() {
        // A path which begins with "./" or "../" is treated as relative
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("./martin#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("./martin#dave")),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("./martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("./martin#dave")),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_str("../martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("../martin#dave")),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("../martin#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("../martin#dave")),
            AssetPath::from("martin#dave")
        );
    }

    #[test]
    fn test_resolve_trailing_slash() {
        // A path which begins with "./" or "../" is treated as relative
        let base = AssetPath::from("alice/bob/");
        assert_eq!(
            base.resolve_str("./martin#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("./martin#dave")),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("./martin#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("./martin#dave")),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_str("../martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("../martin#dave")),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("../martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("../martin#dave")),
            AssetPath::from("alice/martin#dave")
        );
    }

    #[test]
    fn test_resolve_canonicalize() {
        // Test that ".." and "." are removed after concatenation.
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("./martin/stephan/..#dave")),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("./martin/stephan/..#dave")),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_str("../martin/.#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("../martin/.#dave")),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("../martin/.#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("../martin/.#dave")),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_str("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/martin/stephan/..#dave")),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/martin/stephan/..#dave")),
            AssetPath::from("martin#dave")
        );
    }

    #[test]
    fn test_resolve_canonicalize_base() {
        // Test that ".." and "." are removed after concatenation even from the base path.
        let base = AssetPath::from("alice/../bob#carol");
        assert_eq!(
            base.resolve_str("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("bob/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("./martin/stephan/..#dave")),
            AssetPath::from("bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("./martin/stephan/..#dave")),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_str("../martin/.#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("../martin/.#dave")),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("../martin/.#dave").unwrap(),
            AssetPath::from("../martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("../martin/.#dave")),
            AssetPath::from("../martin#dave")
        );
        assert_eq!(
            base.resolve_str("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/martin/stephan/..#dave")),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/martin/stephan/..#dave")),
            AssetPath::from("martin#dave")
        );
    }

    #[test]
    fn test_resolve_canonicalize_with_source() {
        // Test that ".." and "." are removed after concatenation.
        let base = AssetPath::from("source://alice/bob#carol");
        assert_eq!(
            base.resolve_str("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("./martin/stephan/..#dave")),
            AssetPath::from("source://alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("./martin/stephan/..#dave")),
            AssetPath::from("source://alice/martin#dave")
        );
        assert_eq!(
            base.resolve_str("../martin/.#dave").unwrap(),
            AssetPath::from("source://alice/martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("../martin/.#dave")),
            AssetPath::from("source://alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("../martin/.#dave").unwrap(),
            AssetPath::from("source://martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("../martin/.#dave")),
            AssetPath::from("source://martin#dave")
        );
        assert_eq!(
            base.resolve_str("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://martin#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/martin/stephan/..#dave")),
            AssetPath::from("source://martin#dave")
        );
        assert_eq!(
            base.resolve_embed_str("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://martin#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/martin/stephan/..#dave")),
            AssetPath::from("source://martin#dave")
        );
    }

    #[test]
    fn test_resolve_absolute() {
        // Paths beginning with '/' replace the base path
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("/martin/stephan").unwrap(),
            AssetPath::from("martin/stephan")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/martin/stephan")),
            AssetPath::from("martin/stephan")
        );
        assert_eq!(
            base.resolve_embed_str("/martin/stephan").unwrap(),
            AssetPath::from("martin/stephan")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/martin/stephan")),
            AssetPath::from("martin/stephan")
        );
        assert_eq!(
            base.resolve_str("/martin/stephan#dave").unwrap(),
            AssetPath::from("martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/martin/stephan#dave")),
            AssetPath::from("martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve_embed_str("/martin/stephan#dave").unwrap(),
            AssetPath::from("martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/martin/stephan#dave")),
            AssetPath::from("martin/stephan/#dave")
        );
    }

    #[test]
    fn test_resolve_asset_source() {
        // Paths beginning with 'source://' replace the base path
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("source://martin/stephan").unwrap(),
            AssetPath::from("source://martin/stephan")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("source://martin/stephan")),
            AssetPath::from("source://martin/stephan")
        );
        assert_eq!(
            base.resolve_embed_str("source://martin/stephan").unwrap(),
            AssetPath::from("source://martin/stephan")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("source://martin/stephan")),
            AssetPath::from("source://martin/stephan")
        );
        assert_eq!(
            base.resolve_str("source://martin/stephan#dave").unwrap(),
            AssetPath::from("source://martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("source://martin/stephan#dave")),
            AssetPath::from("source://martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve_embed_str("source://martin/stephan#dave")
                .unwrap(),
            AssetPath::from("source://martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("source://martin/stephan#dave")),
            AssetPath::from("source://martin/stephan/#dave")
        );
    }

    #[test]
    fn test_resolve_label() {
        // A relative path with only a label should replace the label portion
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("#dave").unwrap(),
            AssetPath::from("alice/bob#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("#dave")),
            AssetPath::from("alice/bob#dave")
        );
        assert_eq!(
            base.resolve_embed_str("#dave").unwrap(),
            AssetPath::from("alice/bob#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("#dave")),
            AssetPath::from("alice/bob#dave")
        );
    }

    #[test]
    fn test_resolve_insufficient_elements() {
        // Ensure that ".." segments are preserved if there are insufficient elements to remove them.
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve_str("../../joe/next").unwrap(),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("../../joe/next")),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve_embed_str("../../joe/next").unwrap(),
            AssetPath::from("../joe/next")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("../../joe/next")),
            AssetPath::from("../joe/next")
        );
    }

    #[test]
    fn resolve_embed_relative_to_external_path() {
        let base = AssetPath::from("../../a/b.gltf");
        assert_eq!(
            base.resolve_embed_str("c.bin").unwrap(),
            AssetPath::from("../../a/c.bin")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("c.bin")),
            AssetPath::from("../../a/c.bin")
        );
    }

    #[test]
    fn resolve_relative_to_external_path() {
        let base = AssetPath::from("../../a/b.gltf");
        assert_eq!(
            base.resolve_str("c.bin").unwrap(),
            AssetPath::from("../../a/b.gltf/c.bin")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("c.bin")),
            AssetPath::from("../../a/b.gltf/c.bin")
        );
    }

    /*
    "a/b" -> (false, ["a", "b"])
    "/a/b" -> (true, ["a", "b"])
    "C:file" -> (false, ["C:file"]) (no split on :)
    "a\\b" -> (false, ["a\\b"]) (no split on \)
    "a//b" -> ["a","","b"]
    */

    #[test]
    fn test_split_asset_path_segments() {
        assert_eq!(split_asset_path_segments("a/b"), (false, vec!["a", "b"]));
        assert_eq!(split_asset_path_segments("/a/b"), (true, vec!["a", "b"]));
        assert_eq!(split_asset_path_segments("C:file"), (false, vec!["C:file"]));
        assert_eq!(split_asset_path_segments("a\\b"), (false, vec!["a\\b"]));
        assert_eq!(
            split_asset_path_segments("a//b"),
            (false, vec!["a", "", "b"])
        );
    }

    #[test]
    fn test_normalize_asset_path_segments() {
        assert_eq!(
            normalize_asset_path_segments(&["a", ".", "b"], false),
            vec!["a", "b"]
        );
        assert_eq!(
            normalize_asset_path_segments(&["a", "..", "b"], false),
            vec!["b"]
        );
        assert_eq!(
            normalize_asset_path_segments(&["a", "b", ".."], false),
            vec!["a"]
        );
        assert_eq!(
            normalize_asset_path_segments(&["..", "a"], false),
            vec!["..", "a"]
        );
        assert!(normalize_asset_path_segments(&["a", ".."], true).is_empty());
        assert_eq!(
            normalize_asset_path_segments(&["a", "b", ".."], true),
            vec!["a"]
        );
    }

    #[test]
    fn test_join_and_normalize_asset_path() {
        assert_eq!(
            join_and_normalize_asset_path("a/b", false, "c", false, false),
            "a/b/c"
        );
        assert_eq!(
            join_and_normalize_asset_path("a/b", false, "c", false, true),
            "a/c"
        );
        assert_eq!(
            join_and_normalize_asset_path("a/b/", true, "c", false, true),
            "a/b/c"
        );
        assert_eq!(
            join_and_normalize_asset_path("a/b", false, "x", true, false),
            "x"
        );
    }

    //Regression tests: segment-based resolver (no PathBuf)

    #[test]
    fn test_resolve_colon_in_segment() {
        // "C:" and "a:b" are normal segments, not drive or scheme.
        let base = AssetPath::parse("a/b");
        let resolved = base.resolve_str("C:file").unwrap();
        assert_eq!(resolved.path().to_str().unwrap(), "a/b/C:file");

        let resolved = base.resolve_str("a:b").unwrap();
        assert_eq!(resolved.path().to_str().unwrap(), "a/b/a:b");
    }

    #[test]
    fn test_resolve_backslash_in_segment() {
        // Backslash is not a separator at the asset-path layer.
        let base = AssetPath::parse("a/b");
        let resolved = base.resolve_str(r"x\y").unwrap();
        assert_eq!(resolved.path().to_str().unwrap(), r"a/b/x\y");

        let resolved = base.resolve_str(r"x\y/z").unwrap();
        assert_eq!(resolved.path().to_str().unwrap(), r"a/b/x\y/z");
    }

    #[test]
    fn test_resolve_rooted_dotdot() {
        // Rooted base: ".." does not escape above root; we pop when possible.
        let base = AssetPath::parse("/a/b");
        assert_eq!(base.resolve_str("../c").unwrap(), AssetPath::from("/a/c"));

        // "/a" + "../b": ".." pops "a" -> root, then "b" -> "/b".
        let base = AssetPath::parse("/a");
        assert_eq!(base.resolve_str("../b").unwrap(), AssetPath::from("/b"));
    }

    #[test]
    fn test_resolve_multiple_slashes() {
        // "a//b" preserves the empty segment.
        let base = AssetPath::parse("x");
        let resolved = base.resolve_str("a//b").unwrap();
        assert_eq!(resolved.path().to_str().unwrap(), "x/a//b");
    }

    #[test]
    fn test_get_extension() {
        let result = AssetPath::from("http://a.tar.gz#Foo");
        assert_eq!(result.get_full_extension(), Some("tar.gz".to_string()));

        let result = AssetPath::from("http://a#Foo");
        assert_eq!(result.get_full_extension(), None);

        let result = AssetPath::from("http://a.tar.bz2?foo=bar#Baz");
        assert_eq!(result.get_full_extension(), Some("tar.bz2".to_string()));

        let result = AssetPath::from("asset.Custom");
        assert_eq!(result.get_full_extension(), Some("Custom".to_string()));
    }
}
