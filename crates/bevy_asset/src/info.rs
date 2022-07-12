use crate::{path::AssetPath, LabelId};
use bevy_utils::{HashMap, HashSet, Uuid};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata for an asset source.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMeta {
    /// A collection of asset metadata.
    pub assets: Vec<AssetMeta>,
}

/// Metadata for an asset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetMeta {
    /// Asset label.
    pub label: Option<String>,
    /// Asset dependencies.
    pub dependencies: Vec<AssetPath<'static>>,
    /// An unique identifier for an asset type.
    pub type_uuid: Uuid,
}

/// Information about an asset source, such as its path, load state and asset metadata.
#[derive(Clone, Debug)]
pub struct SourceInfo {
    /// Metadata for the source.
    pub meta: Option<SourceMeta>,
    /// The path of the source.
    pub path: PathBuf,
    /// A map of assets and their type identifiers.
    pub asset_types: HashMap<LabelId, Uuid>,
    /// The load state of the source.
    pub load_state: LoadState,
    /// A collection to track which assets were sent to their asset storages.
    pub committed_assets: HashSet<LabelId>,
    /// Current version of the source.
    pub version: usize,
}

impl SourceInfo {
    /// Returns `true` if all assets tracked by the source were loaded into their asset storages.
    pub fn is_loaded(&self) -> bool {
        self.meta.as_ref().map_or(false, |meta| {
            self.committed_assets.len() == meta.assets.len()
        })
    }

    /// Gets the type identifier for an asset identified by `label_id`.
    pub fn get_asset_type(&self, label_id: LabelId) -> Option<Uuid> {
        self.asset_types.get(&label_id).cloned()
    }
}

/// The load state of an asset.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LoadState {
    /// The asset has not been loaded.
    NotLoaded,
    /// The asset is in the process of loading.
    Loading,
    /// The asset has been loaded and is living inside an [`Assets`](crate::Assets) collection.
    Loaded,
    /// The asset failed to load.
    Failed,
    /// The asset was previously loaded, however all handles were dropped and the asset was removed
    /// from the [`Assets`](crate::Assets) collection.
    Unloaded,
}
