use alloc::vec::Vec;

use bevy_reflect::TypePath;

use crate::{Asset, UntypedHandle};

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
}
