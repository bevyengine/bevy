use alloc::vec::Vec;

use crate::{Asset, AssetPath, UntypedHandle};
use bevy_reflect::TypePath;

pub struct LoadBatchRequest {
    pub requests: Vec<AssetPath<'static>>,
}

impl LoadBatchRequest {
    pub fn new<T>(requests: Vec<T>) -> Self
    where
        T:Into<AssetPath<'static>>,
    {
        Self {
            requests: requests.into_iter().map(Into::into).collect(),
        }
    }
}

/// A "loaded batch" containing handles for all assets stored in a given [`AssetPath`].
///
/// This is produced by [`AssetServer::load_batch`](crate::prelude::AssetServer::load_batch).
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedBatch {
    /// The handles of all assets stored in the batch.
    #[dependency]
    pub handles: Vec<UntypedHandle>,
}
