mod draw;
mod font;
mod font_atlas;
mod font_atlas_set;
mod font_loader;

pub use draw::*;
pub use font::*;
pub use font_atlas::*;
pub use font_atlas_set::*;
pub use font_loader::*;

pub mod prelude {
    pub use crate::{Font, TextStyle};
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;

#[derive(Default)]
pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<Font>()
            .add_asset::<FontAtlasSet>()
            .init_asset_loader::<FontLoader>();
    }
}
