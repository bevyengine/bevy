use bevy_scene2::{bsn, template_value, Scene};
use bevy_ui::{
    AlignItems, AlignSelf, Display, FlexDirection, JustifyContent, Node, PositionType, UiRect, Val,
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
        }
        ThemeBackgroundColor(tokens::PANE_HEADER_BG)
        ThemeBorderColor(tokens::PANE_HEADER_BORDER)
        ThemeFontColor(tokens::PANE_HEADER_TEXT)
        template_value(RoundedCorners::Top.to_border_radius(4.0))
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
    }
}

/// Divider between groups of widgets in pane headers
pub fn pane_header_divider() -> impl Scene {
    bsn! {
        Node {
            width: Val::Px(1.0),
            align_self: AlignSelf::Stretch,
        }
        [(
            // Because we want to extend the divider into the header padding area, we'll use
            // an absolutely-positioned child.
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(-6.0),
                bottom: Val::Px(-6.0),
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
            padding: UiRect::axes(Val::Px(6.0), Val::Px(6.0)),
        }
        template_value(RoundedCorners::Bottom.to_border_radius(4.0))
    }
}
