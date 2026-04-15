use bevy_ecs::hierarchy::Children;
use bevy_scene::{bsn, Scene};
use bevy_text::FontWeight;
use bevy_ui::{
    px, AlignItems, AlignSelf, Display, FlexDirection, JustifyContent, Node, PositionType, UiRect,
    Val,
};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeFontColor},
    tokens,
};

/// A standard pane
pub fn pane() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
        }
    }
}

/// Pane header
pub fn pane_header() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::axes(Val::Px(6.0), Val::Px(6.0)),
            border: UiRect {
                left: Val::Px(1.0),
                top: Val::Px(1.0),
                right: Val::Px(1.0),
                bottom: Val::Px(0.0),
            },
            min_height: size::HEADER_HEIGHT,
            column_gap: Val::Px(6.0),
            border_radius: {RoundedCorners::Top.to_border_radius(4.0)},
        }
        ThemeBackgroundColor(tokens::PANE_HEADER_BG)
        ThemeBorderColor(tokens::PANE_HEADER_BORDER)
        ThemeFontColor(tokens::PANE_HEADER_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}

/// Vertical divider between groups of widgets in pane headers
pub fn pane_header_divider() -> impl Scene {
    bsn! {
        Node {
            width: Val::Px(1.0),
            align_self: AlignSelf::Stretch,
        }
        Children [(
            // Because we want to extend the divider into the header padding area, we'll use
            // an absolutely-positioned child.
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: {px(-6)},
                bottom: {px(-6)},
            }
            ThemeBackgroundColor(tokens::PANE_HEADER_DIVIDER)
        )]
    }
}

/// Pane body
pub fn pane_body() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: px(4.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(6.0)),
            border_radius: {RoundedCorners::Bottom.to_border_radius(4.0)}
        }
        ThemeBackgroundColor(tokens::PANE_BODY_BG)
    }
}
