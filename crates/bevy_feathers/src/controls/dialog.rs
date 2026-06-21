use bevy_color::{Alpha, Srgba};
use bevy_ecs::{
    event::EntityEvent, hierarchy::Children, observer::On, reflect::ReflectComponent,
    system::Commands,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, bsn_list, on, Scene, SceneComponent, SceneList};
use bevy_text::FontWeight;
use bevy_ui::{
    px, vh, vw, AlignItems, BorderRadius, BoxShadow, Display, FixedNode, FlexDirection,
    GlobalZIndex, JustifyContent, Node, OverrideClip, PositionType, UiRect, Val,
};
use bevy_ui_widgets::{Activate, ModalDialog, ModalDialogBarrier, RequestClose};

use crate::{
    constants::{fonts, icons, size},
    controls::{ButtonVariant, FeathersToolButton},
    display::icon,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// Props used to construct a [`FeathersDialog`] scene.
pub struct FeathersDialogProps {
    /// Content of this dialog box.
    pub contents: Box<dyn SceneList>,
    /// How wide this dialog box should be.
    pub width: Val,
}

impl Default for FeathersDialogProps {
    fn default() -> Self {
        Self {
            contents: Box::new(bsn_list!()),
            width: Val::Auto,
        }
    }
}

/// A modal dialog box
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersDialogProps)]
#[reflect(Component, Clone, Default)]
pub struct FeathersDialog;

impl FeathersDialog {
    /// Scene function for modal dialog.
    pub fn scene(props: FeathersDialogProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(vw(0.05), vh(0.05)),
            }
            ModalDialogBarrier
            FixedNode
            OverrideClip
            GlobalZIndex(99) // One less than menu layer
            Children [
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    border_radius: BorderRadius::all(px(4)),
                    padding: UiRect::all(px(6.0)),
                    border: UiRect::all(px(1.0)),
                    row_gap: px(6.0),
                    width: {props.width},
                }
                ModalDialog
                ThemeBackgroundColor(tokens::DIALOG_BG)
                ThemeBorderColor(tokens::DIALOG_BORDER)
                InheritableThemeTextColor(tokens::DIALOG_TEXT)
                BoxShadow::new(
                    Srgba::BLACK.with_alpha(0.9).into(),
                    px(0),
                    px(0),
                    px(1),
                    px(4),
                )
                Children [
                    {props.contents}
                ]
            ]
        }
    }
}

/// Header section for a modal dialog
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersDialogHeader;

impl FeathersDialogHeader {
    /// Scene function for modal dialog header.
    pub fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(px(6.0)),
            }
            ThemeBackgroundColor(tokens::DIALOG_HEADER_BG)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::HEADER_FONT,
                weight: FontWeight::BOLD,
            }
        }
    }
}

/// Close button for dialog header
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersDialogClose;

impl FeathersDialogClose {
    /// Scene function for dialog close button.
    pub fn scene() -> impl Scene {
        bsn! {
        @FeathersToolButton {
            @variant: ButtonVariant::Plain,
            @caption: bsn! { icon(icons::X) }
        }
        on(|activate: On<Activate>, mut commands: Commands| {
            commands.trigger(RequestClose { source: activate.event_target() });
        })
        }
    }
}

/// Central body section for a modal dialog
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersDialogBody;

impl FeathersDialogBody {
    /// Scene function for modal dialog body.
    pub fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                padding: UiRect::all(px(6.0)),
            }
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}

/// Footer section for a modal dialog
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersDialogFooter;

impl FeathersDialogFooter {
    /// Scene function for modal dialog footer.
    pub fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                column_gap: px(6.0),
                padding: UiRect::all(px(6.0)),
            }
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}
