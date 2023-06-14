use crate as bevy_asset;
use crate::{Asset, UntypedHandle};
use bevy_reflect::TypePath;

#[derive(Asset, TypePath)]
pub struct LoadedFolder {
    #[dependency]
    pub handles: Vec<UntypedHandle>,
}
