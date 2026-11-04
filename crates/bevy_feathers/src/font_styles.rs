//! A framework for inheritable font styles.
use bevy_app::{Propagate, PropagateOver};
use bevy_asset::{AssetServer, Handle};
use bevy_ecs::{
    component::Component,
    lifecycle::Insert,
    observer::On,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Commands, Query},
    template::FromTemplate,
    world::{FromWorld, World},
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::{Font, FontSize, FontSource, FontWeight, TextFont};

use crate::{constants::fonts, theme::ThemedText};

/// A component which, when inserted on an entity, will load the given font and propagate it
/// downward to any child text entity that has the [`ThemedText`] marker.
#[derive(Component, Default, Clone, Debug, Reflect, FromTemplate)]
#[reflect(Component, Default)]
#[require(ThemedText, PropagateOver::<TextFont>)]
pub struct InheritableFont {
    /// The font source. This can be a font family list such as [`fonts::SANS`], which allows
    /// glyphs missing from one font to fall back to the next.
    pub font: FontSource,
    /// The desired font size.
    pub font_size: FontSize,
    /// The desired font weight.
    pub weight: FontWeight,
}

/// Keeps the fonts referenced by family name in [`fonts::SANS`] loaded, so that they are
/// registered in the font collection.
#[derive(Resource)]
pub(crate) struct FeathersFonts(
    #[expect(dead_code, reason = "only held to keep fonts loaded")] Vec<Handle<Font>>,
);

impl FromWorld for FeathersFonts {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self(
            [
                fonts::REGULAR,
                fonts::ITALIC,
                fonts::BOLD,
                fonts::BOLD_ITALIC,
                fonts::KEY_SYMBOLS,
            ]
            .into_iter()
            .map(|path| asset_server.load(path))
            .collect(),
        )
    }
}

/// An observer which looks for changes to the [`InheritableFont`] component on an entity, and
/// propagates downward the font to all participating text entities.
pub(crate) fn on_changed_font(
    insert: On<Insert<InheritableFont>>,
    font_style: Query<&InheritableFont>,
    mut commands: Commands,
) {
    if let Ok(inheritable_font) = font_style.get(insert.entity) {
        commands.entity(insert.entity).insert(Propagate(TextFont {
            font: inheritable_font.font.clone(),
            font_size: inheritable_font.font_size,
            weight: inheritable_font.weight,
            ..Default::default()
        }));
    }
}
