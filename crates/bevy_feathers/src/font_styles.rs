//! A framework for inheritable font styles.
use bevy_app::{Propagate, PropagateOver};
use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    lifecycle::Insert,
    observer::On,
    reflect::ReflectComponent,
    system::{Commands, Query},
    template::FromTemplate,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::{Font, FontSize, FontWeight, TextFont};

use crate::theme::ThemedText;

/// A component which, when inserted on an entity, will load the given font and propagate it
/// downward to any child text entity that has the [`ThemedText`] marker.
#[derive(Component, Default, Clone, Debug, Reflect, FromTemplate)]
#[reflect(Component, Default)]
#[require(ThemedText, PropagateOver::<TextFont>)]
pub struct InheritableFont {
    /// The font handle.
    pub font: Handle<Font>,
    /// The desired font size.
    pub font_size: FontSize,
    /// The desired font weight.
    pub weight: FontWeight,
}

/// An observer which looks for changes to the [`InheritableFont`] component on an entity, and
/// propagates downward the font to all participating text entities.
pub(crate) fn on_changed_font(
    insert: On<Insert, InheritableFont>,
    font_style: Query<&InheritableFont>,
    mut commands: Commands,
) {
    if let Ok(inheritable_font) = font_style.get(insert.entity) {
        commands.entity(insert.entity).insert(Propagate(TextFont {
            font: inheritable_font.font.clone().into(),
            font_size: inheritable_font.font_size,
            weight: inheritable_font.weight,
            ..Default::default()
        }));
    }
}
