use alloc::vec::Vec;

use crate::{Asset, UntypedHandle};
use bevy_reflect::TypePath;

pub struct LoadBatchRequest {
    pub requests: Vec<&'static str>,
}

impl LoadBatchRequest {
    pub fn new(requests: Vec<&'static str>) -> Self {
        Self { requests }
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
