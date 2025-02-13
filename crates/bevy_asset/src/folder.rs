use alloc::vec::Vec;

use crate::{Asset, UntypedHandle};
use bevy_reflect::TypePath;

/// A "loaded folder" containing handles for all assets stored in a given [`AssetPath`].
///
/// This is produced by [`AssetServer::load_folder`](crate::prelude::AssetServer::load_folder).
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedFolder {
    #[dependency]
    pub handles: Vec<UntypedHandle>,
}
