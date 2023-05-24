use crate::{self as bevy_asset, AssetDependencyVisitor};
use crate::{loader::AssetLoader, processor::Process, Asset, AssetPath};
use downcast_rs::{impl_downcast, Downcast};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

pub const META_FORMAT_VERSION: &str = "1.0";

#[derive(Serialize, Deserialize)]
pub struct AssetMeta<L: AssetLoader, P: Process> {
    pub meta_format_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_info: Option<ProcessedInfo>,
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
}

#[derive(Serialize, Deserialize)]
pub enum AssetAction<LoaderSettings, ProcessSettings> {
    Load {
        loader: String,
        settings: LoaderSettings,
    },
    Process {
        processor: String,
        settings: ProcessSettings,
    },
    Ignore,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ProcessedInfo {
    /// A hash of the asset bytes and the asset .meta data
    pub hash: u64,
    /// A hash of the asset bytes, the asset .meta data, and the `full_hash` of every process_dependency
    pub full_hash: u64,
    pub process_dependencies: Vec<ProcessDependencyInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessDependencyInfo {
    pub full_hash: u64,
    pub path: AssetPath<'static>,
}

/// This exists to
#[derive(Serialize, Deserialize)]
pub struct AssetMetaMinimal {
    pub asset: AssetActionMinimal,
}

#[derive(Serialize, Deserialize)]
pub enum AssetActionMinimal {
    Load { loader: String },
    Process { processor: String },
    Ignore,
}

#[derive(Serialize, Deserialize)]
pub struct AssetMetaProcessedInfoMinimal {
    pub processed_info: Option<ProcessedInfo>,
}

pub trait AssetMetaDyn: Downcast + Send + Sync {
    fn loader_settings(&self) -> Option<&dyn Settings>;
    fn serialize(&self) -> Vec<u8>;
    fn processed_info(&self) -> &Option<ProcessedInfo>;
    fn processed_info_mut(&mut self) -> &mut Option<ProcessedInfo>;
}

impl<L: AssetLoader, P: Process> AssetMetaDyn for AssetMeta<L, P> {
    fn serialize(&self) -> Vec<u8> {
        ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .expect("type is convertible to ron")
            .into_bytes()
    }
    fn loader_settings(&self) -> Option<&dyn Settings> {
        if let AssetAction::Load { settings, .. } = &self.asset {
            Some(settings)
        } else {
            None
        }
    }
    fn processed_info(&self) -> &Option<ProcessedInfo> {
        &self.processed_info
    }
    fn processed_info_mut(&mut self) -> &mut Option<ProcessedInfo> {
        &mut self.processed_info
    }
}

impl_downcast!(AssetMetaDyn);

pub trait Settings: Downcast + Send + Sync + 'static {}

impl<T: 'static> Settings for T where T: Send + Sync {}

impl_downcast!(Settings);

/// The () processor should never be called. This implementation exists to make the meta format nicer to work with.
impl Process for () {
    type Asset = ();
    type Settings = ();
    type OutputLoader = ();

    fn process<'a>(
        &'a self,
        _context: &'a mut bevy_asset::processor::ProcessContext,
        _meta: AssetMeta<(), Self>,
        _writer: &'a mut bevy_asset::io::Writer,
    ) -> bevy_utils::BoxedFuture<'a, Result<(), bevy_asset::processor::ProcessError>> {
        unreachable!()
    }
}

impl Asset for () {}

impl AssetDependencyVisitor for () {
    fn visit_dependencies(&self, _visit: &mut impl FnMut(bevy_asset::UntypedHandle)) {
        unreachable!()
    }
}

/// The () loader should never be called. This implementation exists to make the meta format nicer to work with.
impl AssetLoader for () {
    type Asset = ();
    type Settings = ();
    fn load<'a>(
        &'a self,
        _reader: &'a mut crate::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut crate::LoadContext,
    ) -> bevy_utils::BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
        unreachable!();
    }

    fn extensions(&self) -> &[&str] {
        unreachable!();
    }
}
