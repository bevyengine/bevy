use bevy_app::Propagate;
use bevy_ecs::template::template;
use bevy_scene::{bsn, Scene};
use bevy_text::FontWeight;
use bevy_ui::{px, AlignItems, Display, FlexDirection, JustifyContent, Node, UiRect};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    theme::{
        InheritableThemeTextColor, SurfaceLevel, ThemeBackgroundColor, ThemeBorderColor,
        ThemeContext,
    },
    tokens,
};

/// Group
pub fn group() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            border: px(1),
            border_radius: px(4),
        }
        ThemeBackgroundColor(tokens::GROUP_BG)
        ThemeBorderColor(tokens::GROUP_BORDER)
        template(|_| Ok(Propagate(ThemeContext(SurfaceLevel::Highest))))
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
            padding: UiRect::horizontal(px(10)),
            min_height: size::HEADER_HEIGHT,
            column_gap: px(4),
        }
        InheritableThemeTextColor(tokens::GROUP_HEADER_TEXT)
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
            row_gap: px(4),
            padding: px(6),
        }
        InheritableThemeTextColor(tokens::GROUP_HEADER_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}
