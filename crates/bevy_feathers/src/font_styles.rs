//! A framework for inheritable font styles.
use bevy_app::{Propagate, PropagateOver};
use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    lifecycle::Insert,
    observer::On,
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::{Font, FontSize, FontSource, FontWeight, TextFont};

use crate::theme::ThemedText;

/// A component which, when inserted on an entity, will load the given font and propagate it
/// downward to any child text entity that has the [`ThemedText`] marker.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component, Default)]
#[require(ThemedText, PropagateOver::<TextFont>::default())]
pub struct InheritableFont(pub TextFont);

impl InheritableFont {
    /// Create a new `InheritableFont` from a handle.
    pub fn from_handle(handle: Handle<Font>) -> Self {
        Self(TextFont {
            font: FontSource::Handle(handle),
            font_size: FontSize::Px(16.0),
            weight: FontWeight::NORMAL,
            ..Default::default()
        })
    }
}

/// An observer which looks for changes to the [`InheritableFont`] component on an entity, and
/// propagates downward the font to all participating text entities.
pub(crate) fn on_changed_font(
    insert: On<Insert, InheritableFont>,
    font_style: Query<&InheritableFont>,
    mut commands: Commands,
) {
    if let Ok(InheritableFont(font)) = font_style.get(insert.entity) {
        commands
            .entity(insert.entity)
            .insert(Propagate(font.clone()));
    }
}
