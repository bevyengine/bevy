use crate::io::AssetSourceId;
use bevy_reflect::{
    std_traits::ReflectDefault, utility::NonGenericTypeInfoCell, FromReflect, FromType,
    GetTypeRegistration, Reflect, ReflectDeserialize, ReflectFromPtr, ReflectFromReflect,
    ReflectMut, ReflectOwned, ReflectRef, ReflectSerialize, TypeInfo, TypePath, TypeRegistration,
    Typed, ValueInfo,
};
use bevy_utils::CowArc;
use serde::{de::Visitor, Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    ops::Deref,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Represents a path to an asset in a "virtual filesystem".
///
/// Asset paths consist of three main parts:
/// * [`AssetPath::source`]: The name of the [`AssetSource`](crate::io::AssetSource) to load the asset from.
///     This is optional. If one is not set the default source will be used (which is the `assets` folder by default).
/// * [`AssetPath::path`]: The "virtual filesystem path" pointing to an asset source file.
/// * [`AssetPath::label`]: An optional "named sub asset". When assets are loaded, they are
/// allowed to load "sub assets" of any type, which are identified by a named "label".
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
#[derive(Eq, PartialEq, Hash, Clone, Default)]
pub struct AssetPath<'a> {
    source: AssetSourceId<'a>,
    path: CowArc<'a, Path>,
    label: Option<CowArc<'a, str>>,
}

impl<'a> Debug for AssetPath<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<'a> Display for AssetPath<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParseAssetPathError {
    #[error("Asset source must be followed by '://'")]
    InvalidSourceSyntax,
    #[error("Asset source must be at least one character. Either specify the source before the '://' or remove the `://`")]
    MissingSource,
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
        let (source, path, label) = Self::parse_internal(asset_path).unwrap();
        Ok(Self {
            source: match source {
                Some(source) => AssetSourceId::Name(CowArc::Borrowed(source)),
                None => AssetSourceId::Default,
            },
            path: CowArc::Borrowed(path),
            label: label.map(CowArc::Borrowed),
        })
    }

    fn parse_internal(
        asset_path: &str,
    ) -> Result<(Option<&str>, &Path, Option<&str>), ParseAssetPathError> {
        let mut chars = asset_path.char_indices();
        let mut source_range = None;
        let mut path_range = 0..asset_path.len();
        let mut label_range = None;
        while let Some((index, char)) = chars.next() {
            match char {
                ':' => {
                    let (_, char) = chars
                        .next()
                        .ok_or(ParseAssetPathError::InvalidSourceSyntax)?;
                    if char != '/' {
                        return Err(ParseAssetPathError::InvalidSourceSyntax);
                    }
                    let (index, char) = chars
                        .next()
                        .ok_or(ParseAssetPathError::InvalidSourceSyntax)?;
                    if char != '/' {
                        return Err(ParseAssetPathError::InvalidSourceSyntax);
                    }
                    source_range = Some(0..index - 2);
                    path_range.start = index + 1;
                }
                '#' => {
                    path_range.end = index;
                    label_range = Some(index + 1..asset_path.len());
                    break;
                }
                _ => {}
            }
        }

        let source = match source_range {
            Some(source_range) => {
                if source_range.is_empty() {
                    return Err(ParseAssetPathError::MissingSource);
                }
                Some(&asset_path[source_range])
            }
            None => None,
        };
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
    pub fn source(&self) -> &AssetSourceId {
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
    /// [`Arc`]: std::sync::Arc
    pub fn into_owned(self) -> AssetPath<'static> {
        AssetPath {
            source: self.source.into_owned(),
            path: self.path.into_owned(),
            label: self.label.map(|l| l.into_owned()),
        }
    }

    /// Clones this into an "owned" value. If internally a value is borrowed, it will be cloned into an "owned [`Arc`]".
    /// If internally a value is a static reference, the static reference will be used unchanged.
    /// If internally a value is an "owned [`Arc`]", the [`Arc`] will be cloned.
    ///
    /// [`Arc`]: std::sync::Arc
    #[inline]
    pub fn clone_owned(&self) -> AssetPath<'static> {
        self.clone().into_owned()
    }

    /// Resolves a relative asset path via concatenation. The result will be an `AssetPath` which
    /// is resolved relative to this "base" path.
    ///
    /// ```rust
    /// # use bevy_asset::AssetPath;
    /// assert_eq!(AssetPath::parse("a/b").resolve("c"), Ok(AssetPath::parse("a/b/c")));
    /// assert_eq!(AssetPath::parse("a/b").resolve("./c"), Ok(AssetPath::parse("a/b/c")));
    /// assert_eq!(AssetPath::parse("a/b").resolve("../c"), Ok(AssetPath::parse("a/c")));
    /// assert_eq!(AssetPath::parse("a/b").resolve("c.png"), Ok(AssetPath::parse("a/b/c.png")));
    /// assert_eq!(AssetPath::parse("a/b").resolve("/c"), Ok(AssetPath::parse("c")));
    /// assert_eq!(AssetPath::parse("a/b.png").resolve("#c"), Ok(AssetPath::parse("a/b.png#c")));
    /// assert_eq!(AssetPath::parse("a/b.png#c").resolve("#d"), Ok(AssetPath::parse("a/b.png#d")));
    /// ```
    ///
    /// There are several cases:
    ///
    /// If the `path` argument begins with `#`, then it is considered an asset label, in which case
    /// the result is the base path with the label portion replaced.
    ///
    /// If the path argument begins with '/', then it is considered a 'full' path, in which
    /// case the result is a new `AssetPath` consisting of the base path asset source
    /// (if there is one) with the path and label portions of the relative path. Note that a 'full'
    /// asset path is still relative to the asset source root, and not necessarily an absolute
    /// filesystem path.
    ///
    /// If the `path` argument begins with an asset source (ex: `http://`) then the entire base
    /// path is replaced - the result is the source, path and label (if any) of the `path`
    /// argument.
    ///
    /// Otherwise, the `path` argument is considered a relative path. The result is concatenated
    /// using the following algorithm:
    ///
    /// * The base path and the `path` argument are concatenated.
    /// * Path elements consisting of "/." or "&lt;name&gt;/.." are removed.
    ///
    /// If there are insufficient segments in the base path to match the ".." segments,
    /// then any left-over ".." segments are left as-is.
    pub fn resolve(&self, path: &str) -> Result<AssetPath<'static>, ParseAssetPathError> {
        self.resolve_internal(path, false)
    }

    /// Resolves an embedded asset path via concatenation. The result will be an `AssetPath` which
    /// is resolved relative to this path. This is similar in operation to `resolve`, except that
    /// the the 'file' portion of the base path (that is, any characters after the last '/')
    /// is removed before concatenation, in accordance with the behavior specified in
    /// IETF RFC 1808 "Relative URIs".
    ///
    /// The reason for this behavior is that embedded URIs which start with "./" or "../" are
    /// relative to the *directory* containing the asset, not the asset file. This is consistent
    /// with the behavior of URIs in `JavaScript`, CSS, HTML and other web file formats. The
    /// primary use case for this method is resolving relative paths embedded within asset files,
    /// which are relative to the asset in which they are contained.
    ///
    /// ```rust
    /// # use bevy_asset::AssetPath;
    /// assert_eq!(AssetPath::parse("a/b").resolve_embed("c"), Ok(AssetPath::parse("a/c")));
    /// assert_eq!(AssetPath::parse("a/b").resolve_embed("./c"), Ok(AssetPath::parse("a/c")));
    /// assert_eq!(AssetPath::parse("a/b").resolve_embed("../c"), Ok(AssetPath::parse("c")));
    /// assert_eq!(AssetPath::parse("a/b").resolve_embed("c.png"), Ok(AssetPath::parse("a/c.png")));
    /// assert_eq!(AssetPath::parse("a/b").resolve_embed("/c"), Ok(AssetPath::parse("c")));
    /// assert_eq!(AssetPath::parse("a/b.png").resolve_embed("#c"), Ok(AssetPath::parse("a/b.png#c")));
    /// assert_eq!(AssetPath::parse("a/b.png#c").resolve_embed("#d"), Ok(AssetPath::parse("a/b.png#d")));
    /// ```
    pub fn resolve_embed(&self, path: &str) -> Result<AssetPath<'static>, ParseAssetPathError> {
        self.resolve_internal(path, true)
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
            let mut base_path = PathBuf::from(self.path());
            if replace && !self.path.to_str().unwrap().ends_with('/') {
                // No error if base is empty (per RFC 1808).
                base_path.pop();
            }

            // Strip off leading slash
            let mut is_absolute = false;
            let rpath = match rpath.strip_prefix("/") {
                Ok(p) => {
                    is_absolute = true;
                    p
                }
                _ => rpath,
            };

            let mut result_path = PathBuf::new();
            if !is_absolute && source.is_none() {
                for elt in base_path.iter() {
                    if elt == "." {
                        // Skip
                    } else if elt == ".." {
                        if !result_path.pop() {
                            // Preserve ".." if insufficient matches (per RFC 1808).
                            result_path.push(elt);
                        }
                    } else {
                        result_path.push(elt);
                    }
                }
            }

            for elt in rpath.iter() {
                if elt == "." {
                    // Skip
                } else if elt == ".." {
                    if !result_path.pop() {
                        // Preserve ".." if insufficient matches (per RFC 1808).
                        result_path.push(elt);
                    }
                } else {
                    result_path.push(elt);
                }
            }

            Ok(AssetPath {
                source: match source {
                    Some(source) => AssetSourceId::Name(CowArc::Owned(source.into())),
                    None => self.source.clone_owned(),
                },
                path: CowArc::Owned(result_path.into()),
                label: rlabel.map(|l| CowArc::Owned(l.into())),
            })
        }
    }

    /// Returns the full extension (including multiple '.' values).
    /// Ex: Returns `"config.ron"` for `"my_asset.config.ron"`
    pub fn get_full_extension(&self) -> Option<String> {
        let file_name = self.path().file_name()?.to_str()?;
        let index = file_name.find('.')?;
        let extension = file_name[index + 1..].to_lowercase();
        Some(extension)
    }

    pub(crate) fn iter_secondary_extensions(full_extension: &str) -> impl Iterator<Item = &str> {
        full_extension.chars().enumerate().filter_map(|(i, c)| {
            if c == '.' {
                Some(&full_extension[i + 1..])
            } else {
                None
            }
        })
    }
}

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

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
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

// NOTE: We manually implement "reflect value" because deriving Reflect on `AssetPath` breaks dynamic linking
// See https://github.com/bevyengine/bevy/issues/9747
// NOTE: This could use `impl_reflect_value` if it supported static lifetimes.

impl GetTypeRegistration for AssetPath<'static> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
        registration.insert::<ReflectSerialize>(FromType::<Self>::from_type());
        registration.insert::<ReflectDeserialize>(FromType::<Self>::from_type());
        registration.insert::<ReflectDefault>(FromType::<Self>::from_type());
        registration
    }
}

impl TypePath for AssetPath<'static> {
    fn type_path() -> &'static str {
        "bevy_asset::path::AssetPath<'static>"
    }
    fn short_type_path() -> &'static str {
        "AssetPath<'static>"
    }
    fn type_ident() -> Option<&'static str> {
        Some("AssetPath<'static>")
    }
    fn crate_name() -> Option<&'static str> {
        Option::None
    }
    fn module_path() -> Option<&'static str> {
        Option::None
    }
}
impl Typed for AssetPath<'static> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| {
            let info = ValueInfo::new::<Self>();
            TypeInfo::Value(info)
        })
    }
}
impl Reflect for AssetPath<'static> {
    #[inline]
    fn type_name(&self) -> &str {
        ::core::any::type_name::<Self>()
    }
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }
    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn ::core::any::Any> {
        self
    }
    #[inline]
    fn as_any(&self) -> &dyn ::core::any::Any {
        self
    }
    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
        self
    }
    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }
    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }
    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }
    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }
    #[inline]
    fn apply(&mut self, value: &dyn Reflect) {
        let value = Reflect::as_any(value);
        if let Some(value) = value.downcast_ref::<Self>() {
            *self = value.clone();
        } else {
            panic!("Value is not {}.", std::any::type_name::<Self>());
        }
    }
    #[inline]
    fn set(
        &mut self,
        value: Box<dyn bevy_reflect::Reflect>,
    ) -> Result<(), Box<dyn bevy_reflect::Reflect>> {
        *self = <dyn bevy_reflect::Reflect>::take(value)?;
        Ok(())
    }
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Value(self)
    }
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Value(self)
    }
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Value(self)
    }
    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = bevy_reflect::utility::reflect_hasher();
        Hash::hash(&::core::any::Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(Hasher::finish(&hasher))
    }
    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        let value = <dyn Reflect>::as_any(value);
        if let Some(value) = <dyn ::core::any::Any>::downcast_ref::<Self>(value) {
            Some(::core::cmp::PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }
    fn debug(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Debug::fmt(self, f)
    }
}
impl FromReflect for AssetPath<'static> {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        Some(Clone::clone(<dyn ::core::any::Any>::downcast_ref::<
            AssetPath<'static>,
        >(<dyn Reflect>::as_any(reflect))?))
    }
}

#[cfg(test)]
mod tests {
    use crate::AssetPath;
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

        let result = AssetPath::parse_internal("http://");
        assert_eq!(result, Ok((Some("http"), Path::new(""), None)));

        let result = AssetPath::parse_internal("://x");
        assert_eq!(result, Err(crate::ParseAssetPathError::MissingSource));

        let result = AssetPath::parse_internal("a/b.test#");
        assert_eq!(result, Err(crate::ParseAssetPathError::MissingLabel));

        let result = AssetPath::parse_internal("http:/");
        assert_eq!(result, Err(crate::ParseAssetPathError::InvalidSourceSyntax));
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
            base.resolve("/joe/next").unwrap(),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve_embed("/joe/next").unwrap(),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve("/joe/next#dave").unwrap(),
            AssetPath::from("joe/next#dave")
        );
        assert_eq!(
            base.resolve_embed("/joe/next#dave").unwrap(),
            AssetPath::from("joe/next#dave")
        );
    }

    #[test]
    fn test_resolve_implicit_relative() {
        // A path with no inital directory separator should be considered relative.
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve("joe/next").unwrap(),
            AssetPath::from("alice/bob/joe/next")
        );
        assert_eq!(
            base.resolve_embed("joe/next").unwrap(),
            AssetPath::from("alice/joe/next")
        );
        assert_eq!(
            base.resolve("joe/next#dave").unwrap(),
            AssetPath::from("alice/bob/joe/next#dave")
        );
        assert_eq!(
            base.resolve_embed("joe/next#dave").unwrap(),
            AssetPath::from("alice/joe/next#dave")
        );
    }

    #[test]
    fn test_resolve_explicit_relative() {
        // A path which begins with "./" or "../" is treated as relative
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve("./martin#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("./martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve("../martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("../martin#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
    }

    #[test]
    fn test_resolve_trailing_slash() {
        // A path which begins with "./" or "../" is treated as relative
        let base = AssetPath::from("alice/bob/");
        assert_eq!(
            base.resolve("./martin#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("./martin#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve("../martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("../martin#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
    }

    #[test]
    fn test_resolve_canonicalize() {
        // Test that ".." and "." are removed after concatenation.
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve("../martin/.#dave").unwrap(),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("../martin/.#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
    }

    #[test]
    fn test_resolve_canonicalize_base() {
        // Test that ".." and "." are removed after concatenation even from the base path.
        let base = AssetPath::from("alice/../bob#carol");
        assert_eq!(
            base.resolve("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve("../martin/.#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed("../martin/.#dave").unwrap(),
            AssetPath::from("../martin#dave")
        );
        assert_eq!(
            base.resolve("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
        assert_eq!(
            base.resolve_embed("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("martin#dave")
        );
    }

    #[test]
    fn test_resolve_canonicalize_with_source() {
        // Test that ".." and "." are removed after concatenation.
        let base = AssetPath::from("source://alice/bob#carol");
        assert_eq!(
            base.resolve("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://alice/bob/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("./martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://alice/martin#dave")
        );
        assert_eq!(
            base.resolve("../martin/.#dave").unwrap(),
            AssetPath::from("source://alice/martin#dave")
        );
        assert_eq!(
            base.resolve_embed("../martin/.#dave").unwrap(),
            AssetPath::from("source://martin#dave")
        );
        assert_eq!(
            base.resolve("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://martin#dave")
        );
        assert_eq!(
            base.resolve_embed("/martin/stephan/..#dave").unwrap(),
            AssetPath::from("source://martin#dave")
        );
    }

    #[test]
    fn test_resolve_absolute() {
        // Paths beginning with '/' replace the base path
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve("/martin/stephan").unwrap(),
            AssetPath::from("martin/stephan")
        );
        assert_eq!(
            base.resolve_embed("/martin/stephan").unwrap(),
            AssetPath::from("martin/stephan")
        );
        assert_eq!(
            base.resolve("/martin/stephan#dave").unwrap(),
            AssetPath::from("martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve_embed("/martin/stephan#dave").unwrap(),
            AssetPath::from("martin/stephan/#dave")
        );
    }

    #[test]
    fn test_resolve_asset_source() {
        // Paths beginning with 'source://' replace the base path
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve("source://martin/stephan").unwrap(),
            AssetPath::from("source://martin/stephan")
        );
        assert_eq!(
            base.resolve_embed("source://martin/stephan").unwrap(),
            AssetPath::from("source://martin/stephan")
        );
        assert_eq!(
            base.resolve("source://martin/stephan#dave").unwrap(),
            AssetPath::from("source://martin/stephan/#dave")
        );
        assert_eq!(
            base.resolve_embed("source://martin/stephan#dave").unwrap(),
            AssetPath::from("source://martin/stephan/#dave")
        );
    }

    #[test]
    fn test_resolve_label() {
        // A relative path with only a label should replace the label portion
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve("#dave").unwrap(),
            AssetPath::from("alice/bob#dave")
        );
        assert_eq!(
            base.resolve_embed("#dave").unwrap(),
            AssetPath::from("alice/bob#dave")
        );
    }

    #[test]
    fn test_resolve_insufficient_elements() {
        // Ensure that ".." segments are preserved if there are insufficient elements to remove them.
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            base.resolve("../../joe/next").unwrap(),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            base.resolve_embed("../../joe/next").unwrap(),
            AssetPath::from("../joe/next")
        );
    }

    #[test]
    fn test_get_extension() {
        let result = AssetPath::from("http://a.tar.gz#Foo");
        assert_eq!(result.get_full_extension(), Some("tar.gz".to_string()));

        let result = AssetPath::from("http://a#Foo");
        assert_eq!(result.get_full_extension(), None);
    }
}
