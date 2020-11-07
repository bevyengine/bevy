use crate::{path::AssetPath, LabelId};
use bevy_utils::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMeta {
    pub assets: Vec<AssetMeta>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetMeta {
    pub label: Option<String>,
    pub dependencies: Vec<AssetPath<'static>>,
    pub type_uuid: Uuid,
}

/// Info about a specific asset, such as its path and its current load state
#[derive(Clone, Debug)]
pub struct SourceInfo {
    pub meta: Option<SourceMeta>,
    pub path: PathBuf,
    pub asset_types: HashMap<LabelId, Uuid>,
    pub load_state: LoadState,
    pub committed_assets: HashSet<LabelId>,
    pub version: usize,
}

impl SourceInfo {
    pub fn is_loaded(&self) -> bool {
        self.meta.as_ref().map_or(false, |meta| {
            self.committed_assets.len() == meta.assets.len()
        })
    }

    pub fn get_asset_type(&self, label_id: LabelId) -> Option<Uuid> {
        self.asset_types.get(&label_id).cloned()
    }
}

/// The load state of an asset
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LoadState {
    NotLoaded,
    Loading,
    Loaded,
    Failed,
}
