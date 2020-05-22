mod font;
mod font_loader;
mod render;

pub use font::*;
pub use font_loader::*;

use bevy_app::{AppBuilder, AppPlugin};
use bevy_asset::AddAsset;

#[derive(Default)]
pub struct TextPlugin;

impl AppPlugin for TextPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<Font>()
            .add_asset_loader::<Font, FontLoader>();
    }
}
