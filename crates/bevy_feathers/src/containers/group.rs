use bevy_scene::{bsn, Scene};
use bevy_text::FontWeight;
use bevy_ui::{px, AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect, Val};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor},
    tokens,
};

/// Group
pub fn group() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
        }
    }
}

/// Group header
pub fn group_header() -> impl Scene {
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
            border_radius: {RoundedCorners::Top.to_border_radius(4.0)}
        }
        ThemeBackgroundColor(tokens::GROUP_HEADER_BG)
        ThemeBorderColor(tokens::GROUP_HEADER_BORDER)
        ThemeFontColor(tokens::GROUP_HEADER_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}

/// Group body
pub fn group_body() -> impl Scene {
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
            row_gap: px(4.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(6.0)),
            border_radius: {RoundedCorners::Bottom.to_border_radius(4.0)}
        }
        ThemeBackgroundColor(tokens::GROUP_BODY_BG)
        ThemeBorderColor(tokens::GROUP_BODY_BORDER)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}
