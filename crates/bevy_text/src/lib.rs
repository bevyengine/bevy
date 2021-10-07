mod draw;
mod error;
mod font;
mod font_atlas;
mod font_atlas_set;
mod font_loader;
mod glyph_brush;
mod pipeline;
mod text;
mod text2d;

pub use draw::*;
pub use error::*;
pub use font::*;
pub use font_atlas::*;
pub use font_atlas_set::*;
pub use font_loader::*;
pub use glyph_brush::*;
pub use pipeline::*;
pub use text::*;
pub use text2d::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{Font, Text, Text2dBundle, TextAlignment, TextError, TextSection, TextStyle};
    #[doc(hidden)]
    pub use glyph_brush_layout::{HorizontalAlign, VerticalAlign};
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::entity::Entity;
use bevy_render::RenderStage;

pub type DefaultTextPipeline = TextPipeline<Entity>;

#[derive(Default)]
pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Font>()
            .add_asset::<FontAtlasSet>()
            .init_asset_loader::<FontLoader>()
            .init_resource::<DefaultTextPipeline>()
            .add_system_to_stage(CoreStage::PostUpdate, text2d_system)
            .add_system_to_stage(RenderStage::Draw, text2d::draw_text2d_system);
    }
}
