use crate as bevy_asset;
use crate::{Asset, UntypedHandle};
use bevy_reflect::TypePath;

/// A "loaded folder" containing handles for all assets stored in a given [`AssetPath`].
///
/// [`AssetPath`]: crate::AssetPath
#[derive(Asset, TypePath)]
pub struct LoadedFolder {
    #[dependency]
    pub handles: Vec<UntypedHandle>,
}
