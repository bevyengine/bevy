//! A framework for inheritable font styles.
use bevy_asset::{AssetServer, Handle};
use bevy_ecs::{
    component::Component,
    hierarchy::Children,
    lifecycle::Insert,
    observer::On,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Query, Res},
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::{Font, TextFont};

use crate::{handle_or_path::HandleOrPath, theme::ThemedText};

/// A component which, when inserted on an entity, will load the given font and propagate it
/// downward to any child text entity that has the [`ThemedText`](ThemedText) marker.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct InheritableFont {
    /// The font handle or path.
    pub font: HandleOrPath<Font>,
    /// The desired font size.
    pub font_size: f32,
}

impl InheritableFont {
    /// Create a new `InheritableFont` from a handle.
    pub fn from_handle(handle: Handle<Font>) -> Self {
        Self {
            font: HandleOrPath::Handle(handle),
            font_size: 16.0,
        }
    }

    /// Create a new `InheritableFont` from a path.
    pub fn from_path(path: &str) -> Self {
        Self {
            font: HandleOrPath::Path(path.to_string()),
            font_size: 16.0,
        }
    }
}

/// An observer which looks for changes to the `InheritableFont` component on an entity, and
/// propagates downward the font to all participating text entities.
pub(crate) fn on_changed_font(
    insert: On<Insert, InheritableFont>,
    q_font_style: Query<&InheritableFont>,
    q_children: Query<&Children>,
    q_themed_text: Query<(), With<ThemedText>>,
    assets: Res<AssetServer>,
    mut commands: Commands,
) {
    if let Ok(style) = q_font_style.get(insert.entity)
        && let Some(font) = match style.font {
            HandleOrPath::Handle(ref h) => Some(h.clone()),
            HandleOrPath::Path(ref p) => Some(assets.load::<Font>(p)),
        }
    {
        q_children
            .iter_descendants(insert.entity)
            .filter(|text_entity| q_themed_text.contains(*text_entity))
            .for_each(|text_entity| {
                commands.entity(text_entity).insert(TextFont {
                    font: font.clone(),
                    font_size: style.font_size,
                    ..Default::default()
                });
            });
    }
}
