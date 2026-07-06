//! BSN Scene for loading images and displaying them as [`ImageNode`]s.
use bevy_ecs::query::{Changed, With};
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{component::Component, system::Query};
use bevy_reflect::Reflect;
use bevy_scene::{bsn, Scene};
use bevy_text::TextColor;
use bevy_ui::{px, widget::ImageNode, Node};

use crate::theme::ThemedText;

/// Marker to tint an icon's `ImageNode` by the text color.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
#[require(ThemedText)]
pub struct ThemedIcon;

/// Template which displays an icon.
pub fn icon(image: &'static str) -> impl Scene {
    bsn! {
        Node {
            height: px(14),
        }
        ImageNode {
            image: image
        }
        ThemedIcon
    }
}

/// Template which displays an icon and doesn't tint it by the text color.
pub fn icon_untinted(image: &'static str) -> impl Scene {
    bsn! {
        Node {
            height: px(14),
        }
        ImageNode {
            image: image
        }
    }
}

pub(crate) fn update_themed_icons(
    mut q_icons: Query<(&mut ImageNode, &TextColor), (With<ThemedIcon>, Changed<TextColor>)>,
) {
    for (mut image, color) in &mut q_icons {
        if image.color != color.0 {
            image.color = color.0;
        }
    }
}
