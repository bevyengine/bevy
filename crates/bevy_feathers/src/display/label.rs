//! BSN scene function for displaying a plain text string in the correct font.
use bevy_ecs::hierarchy::Children;
use bevy_scene::{bsn, template_value, Scene};
use bevy_text::FontWeight;
use bevy_ui::{widget::Text, Node};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    theme::{ThemeFontColor, ThemedText},
    tokens,
};

/// A text label.
pub fn label(text: impl Into<String>) -> impl Scene {
    let text = Text::new(text.into());
    bsn! {
        Node
        ThemeFontColor(tokens::TEXT_MAIN)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        Children [
            template_value(text)
            ThemedText
        ]
    }
}

/// A text label with a dimmed color.
pub fn label_dim(text: impl Into<String>) -> impl Scene {
    let text = Text::new(text.into());
    bsn! {
        Node
        ThemeFontColor(tokens::TEXT_DIM)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        Children [
            template_value(text)
            ThemedText
        ]
    }
}
