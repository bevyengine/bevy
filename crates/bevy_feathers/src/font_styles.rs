//! A framework for inheritable font styles.
use bevy_app::Propagate;
use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    lifecycle::Insert,
    observer::On,
    system::{Commands, Query},
    template::GetTemplate,
};
use bevy_text::{Font, TextFont};

/// A component which, when inserted on an entity, will load the given font and propagate it
/// downward to any child text entity that has the [`ThemedText`](crate::theme::ThemedText) marker.
#[derive(Component, Clone, Debug, GetTemplate)]
pub struct InheritableFont {
    /// The font handle or path.
    pub font: Handle<Font>,
    /// The desired font size.
    pub font_size: f32,
}

/// An observer which looks for changes to the `InheritableFont` component on an entity, and
/// propagates downward the font to all participating text entities.
pub(crate) fn on_changed_font(
    ev: On<Insert, InheritableFont>,
    font_style: Query<&InheritableFont>,
    mut commands: Commands,
) {
    if let Ok(inheritable_font) = font_style.get(ev.target()) {
        commands.entity(ev.target()).insert(Propagate(TextFont {
            font: inheritable_font.font.clone(),
            font_size: inheritable_font.font_size,
            ..Default::default()
        }));
    }
}
