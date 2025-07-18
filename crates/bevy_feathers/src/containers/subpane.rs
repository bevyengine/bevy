use bevy_scene2::{bsn, template_value, Scene};
use bevy_ui::{AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect, Val};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor},
    tokens,
};

/// Sub-pane
pub fn subpane() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
        }
    }
}

/// Sub-pane header
pub fn subpane_header() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            border: UiRect {
                left: Val::Px(1.0),
                top: Val::Px(1.0),
                right: Val::Px(1.0),
                bottom: Val::Px(0.0),
            },
            padding: UiRect::axes(Val::Px(10.0), Val::Px(0.0)),
            min_height: size::HEADER_HEIGHT,
            column_gap: Val::Px(4.0),
        }
        ThemeBackgroundColor(tokens::SUBPANE_HEADER_BG)
        ThemeBorderColor(tokens::SUBPANE_HEADER_BORDER)
        ThemeFontColor(tokens::SUBPANE_HEADER_TEXT)
        template_value(RoundedCorners::Top.to_border_radius(4.0))
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
    }
}

/// Sub-pane body
pub fn subpane_body() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            border: UiRect {
                left: Val::Px(1.0),
                top: Val::Px(0.0),
                right: Val::Px(1.0),
                bottom: Val::Px(1.0),
            },
            padding: UiRect::axes(Val::Px(6.0), Val::Px(6.0)),
        }
        ThemeBackgroundColor(tokens::SUBPANE_BODY_BG)
        ThemeBorderColor(tokens::SUBPANE_BODY_BORDER)
        template_value(RoundedCorners::Bottom.to_border_radius(4.0))
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
    }
}
