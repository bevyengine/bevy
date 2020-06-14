mod font;
mod font_loader;
mod font_atlas;
mod font_atlas_set;
mod draw;

pub use font::*;
pub use font_loader::*;
pub use font_atlas::*;
pub use font_atlas_set::*;
pub use draw::*;


use bevy_app::{AppBuilder, AppPlugin};
use bevy_asset::AddAsset;

#[derive(Default)]
pub struct TextPlugin;

impl AppPlugin for TextPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<Font>()
            .add_asset::<FontAtlasSet>()
            .add_asset_loader::<Font, FontLoader>();
    }
}
