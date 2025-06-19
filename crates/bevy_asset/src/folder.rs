use std::{sync::Arc, vec::Vec};

use crate::{Asset, UntypedHandle};
use bevy_reflect::TypePath;

/// This is use for [`AssetServer::load_folderload_folder_with_batch`](crate::prelude::AssetServer::load_folder_with_batch).
#[derive(Debug, Clone, Copy, Default)]
pub enum LoadFilterKind {
    #[default]
    White, //Allow loading
    Black, //Disallow loading
}

impl LoadFilterKind {
    pub fn apply(&self, expr: bool) -> bool {
        match self {
            LoadFilterKind::White => expr,
            LoadFilterKind::Black => !expr,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoadFilter {
    pub paths: Option<Arc<Vec<&'static str>>>,
    pub paths_kind: LoadFilterKind,
    pub extensions: Option<Arc<Vec<&'static str>>>,
    pub extensions_kind: LoadFilterKind,
}

impl LoadFilter {
    pub fn new(
        paths: Vec<&'static str>,
        paths_kind: LoadFilterKind,
        extensions: Vec<&'static str>,
        extensions_kind: LoadFilterKind,
    ) -> Self {
        Self {
            paths: Some(Arc::new(paths)),
            paths_kind,
            extensions: Some(Arc::new(extensions)),
            extensions_kind,
        }
    }
    pub fn paths(paths: Vec<&'static str>, paths_kind: LoadFilterKind) -> Self {
        Self {
            paths: Some(Arc::new(paths)),
            paths_kind,
            extensions: None,
            ..Default::default()
        }
    }
    pub fn extensions(extensions: Vec<&'static str>, extensions_kind: LoadFilterKind) -> Self {
        Self {
            paths: None,
            extensions: Some(Arc::new(extensions)),
            extensions_kind,
            ..Default::default()
        }
    }
}
/// A "loaded folder" containing handles for all assets stored in a given [`AssetPath`].
///
/// This is produced by [`AssetServer::load_folder`](crate::prelude::AssetServer::load_folder).
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedFolder {
    /// The handles of all assets stored in the folder.
    #[dependency]
    pub handles: Vec<UntypedHandle>,
    /// For filtering files that are required or not required.
    pub load_filter: Option<LoadFilter>,
}
