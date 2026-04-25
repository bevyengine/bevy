use crate::io::AssetSourceId;
use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
};
use atomicow::CowArc;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use core::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::Deref,
};
use serde::{de::Visitor, Deserialize, Serialize};
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
    path: CowArc<'a, str>,
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
        write!(f, "{}", self.path)?;
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
            path: clean_path(path),
            label: label.map(CowArc::Borrowed),
        })
    }

    // Attempts to Parse a &str into an `AssetPath`'s `AssetPath::source`, `AssetPath::path`, and `AssetPath::label` components.
    fn parse_internal(
        asset_path: &str,
    ) -> Result<(Option<&str>, &str, Option<&str>), ParseAssetPathError> {
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
                PATH_SEPARATOR => {
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

        let path = &asset_path[path_range];
        Ok((source, path, label))
    }

    /// Creates a new [`AssetPath`] from a [`String`] path.
    ///
    /// Unlike [`Self::parse`], this **does not** interpret the string: the string is used as the
    /// path unconditionally. Prefer [`Self::parse`] where possible.
    #[inline]
    pub fn from_string_path(string: String) -> AssetPath<'a> {
        AssetPath {
            path: CowArc::Owned(string.into()),
            source: AssetSourceId::Default,
            label: None,
        }
    }

    /// Creates a new [`AssetPath`] from a [`str`] path.
    ///
    /// Unlike [`Self::parse`], this **does not** interpret the string: the string is used as the
    /// path unconditionally. Prefer [`Self::parse`] where possible.
    #[inline]
    pub fn from_str_path(s: &'a str) -> AssetPath<'a> {
        AssetPath {
            path: CowArc::Borrowed(s),
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
    pub fn path(&self) -> &str {
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
        // The root directory doesn't have a parent, so return None.
        if self.path.as_ref() == "" {
            return None;
        }
        let path = match &self.path {
            CowArc::Borrowed(path) => path_parent(path).map(CowArc::Borrowed),
            CowArc::Static(path) => path_parent(path).map(CowArc::Static),
            CowArc::Owned(path) => path_parent(path).map(ToString::to_string).map(Into::into),
        };
        // If there isn't a parent, then the path refers to an entry in the root directory. So the
        // parent of that *is* the root directory.
        let path = path.unwrap_or("".into());
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
            && path.path().is_empty()
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
            && path.path().is_empty()
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
        rpath: &str,
        rlabel: Option<&str>,
    ) -> AssetPath<'static> {
        let base_path = if replace
            && !self.path.ends_with(PATH_SEPARATOR)
            && let Some(parent) = path_parent(self.path.as_ref())
        {
            // No error if base is empty (per RFC 1808).
            parent.to_string()
        } else {
            self.path.to_string()
        };

        // Strip off leading slash
        let mut is_absolute = false;
        let rpath = match rpath.strip_prefix(PATH_SEPARATOR) {
            Some(p) => {
                is_absolute = true;
                p
            }
            None => rpath,
        };

        let mut result_path = if !is_absolute && source.is_none() {
            join_paths(&base_path, rpath)
        } else {
            String::from(rpath)
        };
        result_path = normalize_path(&result_path);

        AssetPath {
            source: match source {
                Some(source) => AssetSourceId::Name(CowArc::Owned(source.into())),
                None => self.source.clone_owned(),
            },
            path: CowArc::Owned(result_path.into()),
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
    pub fn get_full_extension(&self) -> Option<&str> {
        let file_name = path_basename(self.path())?;
        let index = file_name.find('.')?;
        let mut extension = &file_name[index + 1..];

        // Strip off any query parameters
        let query = extension.find('?');
        if let Some(offset) = query {
            extension = &extension[..offset];
        }

        Some(extension)
    }

    /// Returns the extension, excluding multiple `.` values.
    ///
    /// Ex: Returns `"ron"` for `"my_asset.config.ron"`
    ///
    /// Also strips out anything follow a `?` to handle query parameters in URIs.
    pub fn get_extension(&self) -> Option<&str> {
        let full_extension = self.get_full_extension()?;
        Some(match full_extension.rfind(".") {
            None => full_extension,
            Some(index) => &full_extension[(index + 1)..],
        })
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
        let mut depth = 0;
        if self.path.starts_with(PATH_SEPARATOR) {
            return true;
        }
        for component in path_components(self.path.as_ref()) {
            match component {
                "." => {}
                ".." => {
                    if depth == 0 {
                        return true;
                    }
                    depth -= 1;
                }
                _ => {
                    depth += 1;
                }
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
            path: if path.contains('\\') {
                clean_path(path)
            } else {
                CowArc::Static(path)
            },
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

impl<'a, 'b> From<&'a AssetPath<'b>> for AssetPath<'b> {
    fn from(value: &'a AssetPath<'b>) -> Self {
        value.clone()
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
        match AssetPath::try_parse(v) {
            Ok(path) => Ok(path.into_owned()),
            Err(err) => Err(E::custom(err)),
        }
    }
}

/// Normalizes the path by collapsing all occurrences of '.' and '..' dot-segments where possible
/// as per [RFC 1808](https://datatracker.ietf.org/doc/html/rfc1808)
pub(crate) fn normalize_path(path: &str) -> String {
    let mut result_path = String::new();
    for elt in path_components(path) {
        if elt == "." {
            // Skip
        } else if elt == ".." {
            let (index, basename) = match result_path.rfind(PATH_SEPARATOR) {
                None => (0, result_path.as_str()),
                Some(index) => (index, &result_path[(index + 1)..]),
            };
            if basename != ".." && !basename.is_empty() {
                result_path.drain(index..);
            } else {
                // Preserve ".." if insufficient matches (per RFC 1808).
                if !result_path.is_empty() {
                    result_path.push(PATH_SEPARATOR);
                }
                result_path.push_str(elt);
            }
        } else {
            if !result_path.is_empty() {
                result_path.push(PATH_SEPARATOR);
            }
            result_path.push_str(elt);
        }
    }
    result_path
}

pub const PATH_SEPARATOR: char = '/';

/// Splits `path` into its directory and basename.
///
/// This assumes the given path uses `/` has its path delimiter. If the path ends in a slash, the
/// basename will be [`None`]. If the path is absolute (starts with a slash), the directory may be
/// [`None`].
///
/// ```rust
/// # use bevy_asset::split_path;
/// assert_eq!(split_path(""), (None, None));
/// assert_eq!(split_path("foo"), (None, Some("foo")));
/// assert_eq!(split_path("foo/bar"), (Some("foo"), Some("bar")));
/// assert_eq!(split_path("/foo/bar"), (Some("/foo"), Some("bar")));
/// assert_eq!(split_path("/foo/"), (Some("/foo"), None));
/// ```
pub fn split_path<'a>(path: &'a str) -> (Option<&'a str>, Option<&'a str>) {
    let (parent, basename) = match path.rfind(PATH_SEPARATOR) {
        Some(slash) => (&path[..slash], &path[(slash + 1)..]),
        None => ("", path),
    };
    (
        (!parent.is_empty()).then_some(parent),
        (!basename.is_empty()).then_some(basename),
    )
}

/// Returns the directory of `path`.
///
/// This assumes the given path uses `/` has its path delimiter. If the path ends in a slash, the
/// basename will be the empty string. If the path is absolute (starts with a slash), and the path
/// has only one components (e.g., `/blah.txt`), [`None`] is returned.
///
/// ```rust
/// # use bevy_asset::path_parent;
/// assert_eq!(path_parent(""), None);
/// assert_eq!(path_parent("foo"), Some(""));
/// assert_eq!(path_parent("foo/bar"), Some("foo"));
/// assert_eq!(path_parent("/foo/bar"), Some("/foo"));
/// assert_eq!(path_parent("/foo/"), Some("/foo"));
/// ```
pub fn path_parent<'a>(path: &'a str) -> Option<&'a str> {
    let (parent, basename) = split_path(path);
    parent.or_else(|| basename.map(|_| ""))
}

/// Returns the basename of `path`.
///
/// This assumes the given path uses `/` as its path delimiter. If the path ends in a slash,
/// [`None`] is returned.
///
/// ```rust
/// # use bevy_asset::path_basename;
/// assert_eq!(path_basename(""), None);
/// assert_eq!(path_basename("foo"), Some("foo"));
/// assert_eq!(path_basename("foo/bar"), Some("bar"));
/// assert_eq!(path_basename("/foo/bar"), Some("bar"));
/// assert_eq!(path_basename("/foo/"), None);
/// ```
pub fn path_basename<'a>(path: &'a str) -> Option<&'a str> {
    split_path(path).1
}

/// Returns the individual components of `path`.
///
/// This assumes the given path uses `/` as its path delimiter.
///
/// ```rust
/// # use bevy_asset::path_components;
/// let mut components = path_components("/foo/bar");
/// assert_eq!(components.next(), Some("foo"));
/// assert_eq!(components.next(), Some("bar"));
/// assert_eq!(components.next(), None);
/// ```
pub fn path_components<'a>(mut path: &'a str) -> impl Iterator<Item = &'a str> {
    path = path.strip_prefix(PATH_SEPARATOR).unwrap_or(path);
    path = path.strip_suffix(PATH_SEPARATOR).unwrap_or(path);
    path.split(PATH_SEPARATOR)
        .filter(|component| !component.is_empty())
}

/// Returns the ancestors of `path` (including `path`).
///
/// This assumes the given path uses `/` as its path delimiter.
///
/// ```rust
/// # use bevy_asset::path_ancestors;
/// let mut ancestors = path_ancestors("/foo/bar");
/// assert_eq!(ancestors.next(), Some("/foo/bar"));
/// assert_eq!(ancestors.next(), Some("/foo"));
/// assert_eq!(ancestors.next(), Some("/"));
/// assert_eq!(ancestors.next(), None);
///
/// let mut ancestors = path_ancestors("../foo/bar");
/// assert_eq!(ancestors.next(), Some("../foo/bar"));
/// assert_eq!(ancestors.next(), Some("../foo"));
/// assert_eq!(ancestors.next(), Some(".."));
/// assert_eq!(ancestors.next(), None);
/// ```
pub fn path_ancestors<'a>(mut path: &'a str) -> impl Iterator<Item = &'a str> {
    if path != "/" {
        path = path.strip_suffix(PATH_SEPARATOR).unwrap_or(path);
    }
    AncestorIter { path: Some(path) }
}

/// Cleans the given path into a "platform agnostic" path.
///
/// Different platforms have different semantics for their file paths. In particular, Windows allows
/// users to use `\` as a path separator. To address this, we convert the path to a common
/// unambiguous representation - paths are separated by `/`. Since we don't know whether a path was
/// created on Windows or not, we must replace all `\` with `/`. As a consequence, paths on all
/// platforms cannot include `\`.
///
/// ```rust
/// # use bevy_asset::clean_path;
/// # use atomicow::CowArc;
/// assert_eq!(clean_path("/foo/bar"), CowArc::Static("/foo/bar"));
/// assert_eq!(clean_path("C:\\foo\\bar"), CowArc::Owned("C:/foo/bar".into()));
/// assert_eq!(clean_path("/foo\\bar"), CowArc::Owned("/foo/bar".into()));
/// ```
pub fn clean_path<'a>(raw_path: &'a str) -> CowArc<'a, str> {
    if raw_path.find('\\').is_none() {
        return CowArc::Borrowed(raw_path);
    }
    CowArc::Owned(raw_path.replace('\\', "/").into())
}

/// Joins the two paths together into a single path.
///
/// If `rpath` is an absolute path, it is used entirely (`lpath` is ignored).
///
/// ```rust
/// # use bevy_asset::join_paths;
/// assert_eq!(join_paths("foo/bar", "baz/quox"), "foo/bar/baz/quox");
/// assert_eq!(join_paths("foo/bar/", "baz/quox"), "foo/bar/baz/quox");
/// assert_eq!(join_paths("foo/bar", "/baz/quox"), "/baz/quox");
/// assert_eq!(join_paths("", "baz/quox"), "baz/quox");
/// ```
pub fn join_paths(lpath: &str, rpath: &str) -> String {
    if lpath.is_empty() {
        return rpath.to_string();
    }

    if rpath.starts_with(PATH_SEPARATOR) {
        return rpath.to_string();
    }

    if lpath.ends_with(PATH_SEPARATOR) {
        format!("{lpath}{rpath}")
    } else {
        format!("{lpath}/{rpath}")
    }
}

/// Returns whether the path is an absolute path (starts with '/').
///
/// ```rust
/// # use bevy_asset::is_absolute_path;
/// assert!(!is_absolute_path("foo/bar"));
/// assert!(is_absolute_path("/foo/bar"));
/// ```
pub fn is_absolute_path(path: &str) -> bool {
    path.starts_with(PATH_SEPARATOR)
}

/// Returns the file extension for the given path.
///
/// The file extension is defined as the string after the last `.` in the last path component.
/// Returns [`None`] if the last component does not contain a dot (or the dot is at the end of the
/// path).
///
/// ```rust
/// # use bevy_asset::path_file_extension;
/// assert_eq!(path_file_extension("foo/bar.png"), Some("png"));
/// assert_eq!(path_file_extension("foo/bar."), None);
/// assert_eq!(path_file_extension("foo/.png"), Some("png"));
/// assert_eq!(path_file_extension("foo.png"), Some("png"));
/// assert_eq!(path_file_extension("foo.png/ok_thats_odd"), None);
/// ```
pub fn path_file_extension(path: &str) -> Option<&str> {
    let path = path_basename(path).unwrap_or(path);
    let dot = path.rfind('.')?;

    if dot + 1 == path.len() {
        // The dot is the last character, so there's nothing in the extension.
        return None;
    }

    // TODO: Should we strip off query parameters like `get_full_extension`?

    Some(&path[(dot + 1)..])
}

/// Iterator for the ancestors of a file path - including the path itself.
struct AncestorIter<'a> {
    /// The path for which to get the ancestors.
    ///
    /// This is [`None`] if the path has no more ancestors.
    path: Option<&'a str>,
}

impl<'a> Iterator for AncestorIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let path = self.path?;
        if let Some(slash) = path.rfind(PATH_SEPARATOR) {
            self.path = if slash == 0 {
                if path == "/" {
                    None
                } else {
                    Some("/")
                }
            } else {
                Some(&path[..slash])
            };
        } else {
            self.path = None;
        }
        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use crate::AssetPath;

    #[test]
    fn parse_asset_path() {
        let result = AssetPath::parse_internal("a/b.test");
        assert_eq!(result, Ok((None, "a/b.test", None)));

        let result = AssetPath::parse_internal("http://a/b.test");
        assert_eq!(result, Ok((Some("http"), "a/b.test", None)));

        let result = AssetPath::parse_internal("http://a/b.test#Foo");
        assert_eq!(result, Ok((Some("http"), "a/b.test", Some("Foo"))));

        let result = AssetPath::parse_internal("localhost:80/b.test");
        assert_eq!(result, Ok((None, "localhost:80/b.test", None)));

        let result = AssetPath::parse_internal("http://localhost:80/b.test");
        assert_eq!(result, Ok((Some("http"), "localhost:80/b.test", None)));

        let result = AssetPath::parse_internal("http://localhost:80/b.test#Foo");
        assert_eq!(
            result,
            Ok((Some("http"), "localhost:80/b.test", Some("Foo")))
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
        assert_eq!(result, Ok((Some("http"), "", None)));

        let result = AssetPath::parse_internal("://x");
        assert_eq!(result, Err(crate::ParseAssetPathError::MissingSource));

        let result = AssetPath::parse_internal("a/b.test#");
        assert_eq!(result, Err(crate::ParseAssetPathError::MissingLabel));
    }

    #[test]
    fn test_serialize() {
        assert!(ron::de::from_str::<AssetPath>("\"a/b.test\"").is_ok());
        assert!(ron::de::from_str::<AssetPath>("\"a/b.test#\"").is_err());
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
            AssetPath::from("martin/stephan#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("/martin/stephan#dave")),
            AssetPath::from("martin/stephan#dave")
        );
        assert_eq!(
            base.resolve_embed_str("/martin/stephan#dave").unwrap(),
            AssetPath::from("martin/stephan#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("/martin/stephan#dave")),
            AssetPath::from("martin/stephan#dave")
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
            AssetPath::from("source://martin/stephan#dave")
        );
        assert_eq!(
            base.resolve(&AssetPath::parse("source://martin/stephan#dave")),
            AssetPath::from("source://martin/stephan#dave")
        );
        assert_eq!(
            base.resolve_embed_str("source://martin/stephan#dave")
                .unwrap(),
            AssetPath::from("source://martin/stephan#dave")
        );
        assert_eq!(
            base.resolve_embed(&AssetPath::parse("source://martin/stephan#dave")),
            AssetPath::from("source://martin/stephan#dave")
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

    #[test]
    fn test_get_full_extension() {
        let result = AssetPath::from("http://a.tar.gz#Foo");
        assert_eq!(result.get_full_extension(), Some("tar.gz"));

        let result = AssetPath::from("http://a#Foo");
        assert_eq!(result.get_full_extension(), None);

        let result = AssetPath::from("http://a.tar.bz2?foo=bar#Baz");
        assert_eq!(result.get_full_extension(), Some("tar.bz2"));

        let result = AssetPath::from("asset.Custom");
        assert_eq!(result.get_full_extension(), Some("Custom"));
    }

    #[test]
    fn test_get_extension() {
        let result = AssetPath::from("http://a.tar.gz#Foo");
        assert_eq!(result.get_extension(), Some("gz"));

        let result = AssetPath::from("http://a#Foo");
        assert_eq!(result.get_extension(), None);

        let result = AssetPath::from("http://a.tar.bz2?foo=bar#Baz");
        assert_eq!(result.get_extension(), Some("bz2"));

        let result = AssetPath::from("asset.Custom");
        assert_eq!(result.get_extension(), Some("Custom"));
    }
}
