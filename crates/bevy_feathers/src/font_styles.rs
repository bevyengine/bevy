//! A framework for inheritable font styles.
use bevy_app::Propagate;
use bevy_asset::{AssetServer, Handle};
use bevy_ecs::{
    component::Component,
    lifecycle::Insert,
    observer::On,
    system::{Commands, Query, Res},
};
use bevy_text::{Font, TextFont};

use crate::handle_or_path::HandleOrPath;

/// A component which, when inserted on an entity, will load the given font and propoagate it
/// downward to any child text entity that has the `UseTheme` marker.
#[derive(Component, Default, Clone, Debug)]
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
/// propagates downward the text color to all participating text entities.
pub(crate) fn on_changed_font(
    ev: On<Insert, InheritableFont>,
    font_style: Query<&InheritableFont>,
    assets: Res<AssetServer>,
    mut commands: Commands,
) {
    if let Ok(style) = font_style.get(ev.target()) {
        if let Some(font) = match style.font {
            HandleOrPath::Handle(ref h) => Some(h.clone()),
            HandleOrPath::Path(ref p) => Some(assets.load::<Font>(p)),
        } {
            commands.entity(ev.target()).insert(Propagate(TextFont {
                font,
                font_size: style.font_size,
                ..Default::default()
            }));
        }
    }
}
