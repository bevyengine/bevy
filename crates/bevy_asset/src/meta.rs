use crate::{self as bevy_asset, DeserializeMetaError, VisitAssetDependencies};
use crate::{loader::AssetLoader, processor::Process, Asset, AssetPath};
use bevy_utils::tracing::error;
use downcast_rs::{impl_downcast, Downcast};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

pub const META_FORMAT_VERSION: &str = "1.0";
pub type MetaTransform = Box<dyn Fn(&mut dyn AssetMetaDyn) + Send + Sync>;

/// Asset metadata that informs how an [`Asset`] should be handled by the asset system.
///
/// `L` is the [`AssetLoader`] (if one is configured) for the [`AssetAction`]. This can be `()` if it is not required.
/// `P` is the [`Process`] processor, if one is configured for the [`AssetAction`]. This can be `()` if it is not required.
#[derive(Serialize, Deserialize)]
pub struct AssetMeta<L: AssetLoader, P: Process> {
    /// The version of the meta format being used. This will change whenever a breaking change is made to
    /// the meta format.
    pub meta_format_version: String,
    /// Information produced by the [`AssetProcessor`] _after_ processing this asset.
    /// This will only exist alongside processed versions of assets. You should not manually set it in your asset source files.
    ///
    /// [`AssetProcessor`]: crate::processor::AssetProcessor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_info: Option<ProcessedInfo>,
    /// How to handle this asset in the asset system. See [`AssetAction`].
    pub asset: AssetAction<L::Settings, P::Settings>,
}

impl<L: AssetLoader, P: Process> AssetMeta<L, P> {
    pub fn new(asset: AssetAction<L::Settings, P::Settings>) -> Self {
        Self {
            meta_format_version: META_FORMAT_VERSION.to_string(),
            processed_info: None,
            asset,
        }
    }

    /// Deserializes the given serialized byte representation of the asset meta.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeMetaError> {
        Ok(ron::de::from_bytes(bytes)?)
    }
}

/// Configures how an asset source file should be handled by the asset system.
#[derive(Serialize, Deserialize)]
pub enum AssetAction<LoaderSettings, ProcessSettings> {
    /// Load the asset with the given loader and settings
    /// See [`AssetLoader`].
    Load {
        loader: String,
        settings: LoaderSettings,
    },
    /// Process the asset with the given processor and settings.
    /// See [`Process`] and [`AssetProcessor`].
    ///
    /// [`AssetProcessor`]: crate::processor::AssetProcessor
    Process {
        processor: String,
        settings: ProcessSettings,
    },
    /// Do nothing with the asset
    Ignore,
}

/// Info produced by the [`AssetProcessor`] for a given processed asset. This is used to determine if an
/// asset source file (or its dependencies) has changed.
///
/// [`AssetProcessor`]: crate::processor::AssetProcessor
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ProcessedInfo {
    /// A hash of the asset bytes and the asset .meta data
    pub hash: AssetHash,
    /// A hash of the asset bytes, the asset .meta data, and the `full_hash` of every `process_dependency`
    pub full_hash: AssetHash,
    /// Information about the "process dependencies" used to process this asset.
    pub process_dependencies: Vec<ProcessDependencyInfo>,
}

/// Information about a dependency used to process an asset. This is used to determine whether an asset's "process dependency"
/// has changed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessDependencyInfo {
    pub full_hash: AssetHash,
    pub path: AssetPath<'static>,
}

/// This is a minimal counterpart to [`AssetMeta`] that exists to speed up (or enable) serialization in cases where the whole [`AssetMeta`] isn't
/// necessary.
// PERF:
// Currently, this is used when retrieving asset loader and processor information (when the actual type is not known yet). This could probably
// be replaced (and made more efficient) by a custom deserializer that reads the loader/processor information _first_, then deserializes the contents
// using a type registry.
#[derive(Serialize, Deserialize)]
pub struct AssetMetaMinimal {
    pub asset: AssetActionMinimal,
}

/// This is a minimal counterpart to [`AssetAction`] that exists to speed up (or enable) serialization in cases where the whole [`AssetAction`]
/// isn't necessary.
#[derive(Serialize, Deserialize)]
pub enum AssetActionMinimal {
    Load { loader: String },
    Process { processor: String },
    Ignore,
}

/// This is a minimal counterpart to [`ProcessedInfo`] that exists to speed up serialization in cases where the whole [`ProcessedInfo`] isn't
/// necessary.
#[derive(Serialize, Deserialize)]
pub struct ProcessedInfoMinimal {
    pub processed_info: Option<ProcessedInfo>,
}

/// A dynamic type-erased counterpart to [`AssetMeta`] that enables passing around and interacting with [`AssetMeta`] without knowing
/// its type.
pub trait AssetMetaDyn: Downcast + Send + Sync {
    /// Returns a reference to the [`AssetLoader`] settings, if they exist.
    fn loader_settings(&self) -> Option<&dyn Settings>;
    /// Returns a mutable reference to the [`AssetLoader`] settings, if they exist.
    fn loader_settings_mut(&mut self) -> Option<&mut dyn Settings>;
    /// Serializes the internal [`AssetMeta`].
    fn serialize(&self) -> Vec<u8>;
    /// Returns a reference to the [`ProcessedInfo`] if it exists.
    fn processed_info(&self) -> &Option<ProcessedInfo>;
    /// Returns a mutable reference to the [`ProcessedInfo`] if it exists.
    fn processed_info_mut(&mut self) -> &mut Option<ProcessedInfo>;
}

impl<L: AssetLoader, P: Process> AssetMetaDyn for AssetMeta<L, P> {
    fn loader_settings(&self) -> Option<&dyn Settings> {
        if let AssetAction::Load { settings, .. } = &self.asset {
            Some(settings)
        } else {
            None
        }
    }
    fn loader_settings_mut(&mut self) -> Option<&mut dyn Settings> {
        if let AssetAction::Load { settings, .. } = &mut self.asset {
            Some(settings)
        } else {
            None
        }
    }
    fn serialize(&self) -> Vec<u8> {
        ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .expect("type is convertible to ron")
            .into_bytes()
    }
    fn processed_info(&self) -> &Option<ProcessedInfo> {
        &self.processed_info
    }
    fn processed_info_mut(&mut self) -> &mut Option<ProcessedInfo> {
        &mut self.processed_info
    }
}

impl_downcast!(AssetMetaDyn);

/// Settings used by the asset system, such as by [`AssetLoader`], [`Process`], and [`AssetSaver`]
///
/// [`AssetSaver`]: crate::saver::AssetSaver
pub trait Settings: Downcast + Send + Sync + 'static {}

impl<T: 'static> Settings for T where T: Send + Sync {}

impl_downcast!(Settings);

/// The () processor should never be called. This implementation exists to make the meta format nicer to work with.
impl Process for () {
    type Settings = ();
    type OutputLoader = ();

    async fn process<'a>(
        &'a self,
        _context: &'a mut bevy_asset::processor::ProcessContext<'_>,
        _meta: AssetMeta<(), Self>,
        _writer: &'a mut bevy_asset::io::Writer,
    ) -> Result<(), bevy_asset::processor::ProcessError> {
        unreachable!()
    }
}

impl Asset for () {}

impl VisitAssetDependencies for () {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(bevy_asset::UntypedAssetId)) {
        unreachable!()
    }
}

/// The () loader should never be called. This implementation exists to make the meta format nicer to work with.
impl AssetLoader for () {
    type Asset = ();
    type Settings = ();
    type Error = std::io::Error;
    async fn load<'a>(
        &'a self,
        _reader: &'a mut crate::io::Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut crate::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        unreachable!();
    }

    fn extensions(&self) -> &[&str] {
        unreachable!();
    }
}

pub(crate) fn meta_transform_settings<S: Settings>(
    meta: &mut dyn AssetMetaDyn,
    settings: &(impl Fn(&mut S) + Send + Sync + 'static),
) {
    if let Some(loader_settings) = meta.loader_settings_mut() {
        if let Some(loader_settings) = loader_settings.downcast_mut::<S>() {
            settings(loader_settings);
        } else {
            error!(
                "Configured settings type {} does not match AssetLoader settings type",
                std::any::type_name::<S>(),
            );
        }
    }
}

pub(crate) fn loader_settings_meta_transform<S: Settings>(
    settings: impl Fn(&mut S) + Send + Sync + 'static,
) -> MetaTransform {
    Box::new(move |meta| meta_transform_settings(meta, &settings))
}

pub type AssetHash = [u8; 32];

/// NOTE: changing the hashing logic here is a _breaking change_ that requires a [`META_FORMAT_VERSION`] bump.
pub(crate) fn get_asset_hash(meta_bytes: &[u8], asset_bytes: &[u8]) -> AssetHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(meta_bytes);
    hasher.update(asset_bytes);
    *hasher.finalize().as_bytes()
}

/// NOTE: changing the hashing logic here is a _breaking change_ that requires a [`META_FORMAT_VERSION`] bump.
pub(crate) fn get_full_asset_hash(
    asset_hash: AssetHash,
    dependency_hashes: impl Iterator<Item = AssetHash>,
) -> AssetHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&asset_hash);
    for hash in dependency_hashes {
        hasher.update(&hash);
    }
    *hasher.finalize().as_bytes()
}
