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

/// Represents a path to an asset in a "virtual filesystem".
///
/// Asset paths consist of two main parts:
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
/// // This loads the `my_scene.scn` base asset.
/// let scene: Handle<Scene> = asset_server.load("my_scene.scn");
///
/// // This loads the `PlayerMesh` labeled asset from the `my_scene.scn` base asset.
/// let mesh: Handle<Mesh> = asset_server.load("my_scene.scn#PlayerMesh");
/// ```
///
/// [`AssetPath`] implements [`From`] for `&'static str`, `&'static Path`, and `&'a String`,
/// which allows us to optimize the static cases.
/// This means that the common case of `asset_server.load("my_scene.scn")` when it creates and
/// clones internal owned [`AssetPaths`](AssetPath).
/// This also means that you should use [`AssetPath::new`] in cases where `&str` is the explicit type.
#[derive(Eq, PartialEq, Hash, Clone, Default)]
pub struct AssetPath<'a> {
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
        write!(f, "{}", self.path.display())?;
        if let Some(label) = &self.label {
            write!(f, "#{label}")?;
        }
        Ok(())
    }
}

impl<'a> AssetPath<'a> {
    /// Creates a new [`AssetPath`] from a string in the asset path format:
    /// * An asset at the root: `"scene.gltf"`
    /// * An asset nested in some folders: `"some/path/scene.gltf"`
    /// * An asset with a "label": `"some/path/scene.gltf#Mesh0"`
    ///
    /// Prefer [`From<'static str>`] for static strings, as this will prevent allocations
    /// and reference counting for [`AssetPath::into_owned`].
    pub fn new(asset_path: &'a str) -> AssetPath<'a> {
        let (path, label) = Self::get_parts(asset_path);
        Self {
            path: CowArc::Borrowed(path),
            label: label.map(CowArc::Borrowed),
        }
    }

    fn get_parts(asset_path: &str) -> (&Path, Option<&str>) {
        let mut parts = asset_path.splitn(2, '#');
        let path = Path::new(parts.next().expect("Path must be set."));
        let label = parts.next();
        (path, label)
    }

    /// Creates a new [`AssetPath`] from a [`Path`].
    #[inline]
    pub fn from_path(path: impl Into<CowArc<'a, Path>>) -> AssetPath<'a> {
        AssetPath {
            path: path.into(),
            label: None,
        }
    }

    /// Gets the "sub-asset label".
    #[inline]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
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
    pub fn with_label(&self, label: impl Into<CowArc<'a, str>>) -> AssetPath<'a> {
        AssetPath {
            path: self.path.clone(),
            label: Some(label.into()),
        }
    }

    /// Converts this into an "owned" value. If internally a value is borrowed, it will be cloned into an "owned [`Arc`]".
    /// If it is already an "owned [`Arc`]", it will remain unchanged.
    ///
    /// [`Arc`]: std::sync::Arc
    pub fn into_owned(self) -> AssetPath<'static> {
        AssetPath {
            path: self.path.into_owned(),
            label: self.label.map(|l| l.into_owned()),
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
        let (path, label) = Self::get_parts(asset_path);
        AssetPath {
            path: CowArc::Static(path),
            label: label.map(CowArc::Static),
        }
    }
}

impl<'a> From<&'a String> for AssetPath<'a> {
    #[inline]
    fn from(asset_path: &'a String) -> Self {
        AssetPath::new(asset_path.as_str())
    }
}

impl From<String> for AssetPath<'static> {
    #[inline]
    fn from(asset_path: String) -> Self {
        AssetPath::new(asset_path.as_str()).into_owned()
    }
}

impl From<&'static Path> for AssetPath<'static> {
    #[inline]
    fn from(path: &'static Path) -> Self {
        Self {
            path: CowArc::Static(path),
            label: None,
        }
    }
}

impl From<PathBuf> for AssetPath<'static> {
    #[inline]
    fn from(path: PathBuf) -> Self {
        Self {
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
        Ok(AssetPath::new(v).into_owned())
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
