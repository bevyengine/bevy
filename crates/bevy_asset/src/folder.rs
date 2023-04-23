use crate as bevy_asset;
use crate::{Asset, UntypedHandle};

#[derive(Asset)]
pub struct LoadedFolder {
    #[dependency]
    pub handles: Vec<UntypedHandle>,
}
