mod error;
mod font;
mod font_atlas;
mod font_atlas_set;
mod font_loader;
mod glyph_brush;
mod pipeline;
mod text;
mod text2d;

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
    pub use crate::{
        Font, HorizontalAlign, Text, Text2dBundle, TextAlignment, TextError, TextSection,
        TextStyle, VerticalAlign,
    };
}

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::{entity::Entity, schedule::ParallelSystemDescriptorCoercion};
use bevy_render2::{RenderApp, RenderStage};
use bevy_sprite2::SpriteSystem;

pub type DefaultTextPipeline = TextPipeline<Entity>;

#[derive(Default)]
pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Font>()
            .add_asset::<FontAtlasSet>()
            // TODO: uncomment when #2215 is fixed
            // .register_type::<Text>()
            .register_type::<VerticalAlign>()
            .register_type::<HorizontalAlign>()
            .init_asset_loader::<FontLoader>()
            .insert_resource(DefaultTextPipeline::default())
            .add_system_to_stage(CoreStage::PostUpdate, text2d_system);

        let render_app = app.sub_app(RenderApp);
        render_app.add_system_to_stage(
            RenderStage::Extract,
            extract_text2d_sprite.after(SpriteSystem::ExtractSprite),
        );
    }
}
