use bevy_app::{Plugin, PreUpdate};
use bevy_core_widgets::{Activate, CallbackTemplate, CoreButton};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or},
    schedule::IntoScheduleConfigs,
    system::{Commands, In, Query},
};
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_scene2::{prelude::*, template_value};
use bevy_ui::{AlignItems, InteractionDisabled, JustifyContent, Node, Pressed, UiRect, Val};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeFontColor},
    tokens,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_winit::cursor::CursorIcon;

/// Color variants for buttons. This also functions as a component used by the dynamic styling
/// system to identify which entities are buttons.
#[derive(Component, Default, Clone)]
pub enum ButtonVariant {
    /// The standard button appearance
    #[default]
    Normal,
    /// A button with a more prominent color, this is used for "call to action" buttons,
    /// default buttons for dialog boxes, and so on.
    Primary,
    /// For a toggle button, indicates that the button is in a "toggled" state.
    Selected,
    /// Don't display the button background unless hovering or pressed.
    Plain,
}

/// Parameters for the button template, passed to [`button`] function.
#[derive(Default)]
pub struct ButtonProps {
    /// Color variant for the button.
    pub variant: ButtonVariant,
    /// Rounded corners options
    pub corners: RoundedCorners,
    /// Click handler
    pub on_click: CallbackTemplate<In<Activate>>,
}

/// Button scene function.
///
/// # Arguments
/// * `props` - construction properties for the button.
pub fn button(props: ButtonProps) -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            min_width: size::ROW_HEIGHT,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
            flex_grow: 1.0,
        }
        CoreButton {
            on_activate: {props.on_click.clone()},
        }
        template_value(props.variant)
        template_value(props.corners.to_border_radius(4.0))
        Hovered
        // TODO: port CursonIcon to GetTemplate
        // CursorIcon::System(bevy_window::SystemCursorIcon::Pointer)
        TabIndex(0)
        ThemeBackgroundColor(tokens::BUTTON_BG)
        ThemeFontColor(tokens::BUTTON_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
    }
}

/// Tool button scene function: a smaller button for embedding in panel headers.
///
/// # Arguments
/// * `props` - construction properties for the button.
pub fn tool_button(props: ButtonProps) -> impl Scene {
    bsn! {
        Node {
            height: size::TOOL_HEIGHT,
            min_width: size::TOOL_HEIGHT,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(2.0), Val::Px(0.)),
        }
        CoreButton {
            on_activate: {props.on_click.clone()},
        }
        template_value(props.variant)
        template_value(props.corners.to_border_radius(3.0))
        Hovered
        // TODO: port CursonIcon to GetTemplate
        // CursorIcon::System(bevy_window::SystemCursorIcon::Pointer)
        TabIndex(0)
        ThemeBackgroundColor(tokens::BUTTON_BG)
        ThemeFontColor(tokens::BUTTON_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: 14.0,
        }
    }
}

fn update_button_styles(
    q_buttons: Query<
        (
            Entity,
            &ButtonVariant,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeFontColor,
        ),
        Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
    >,
    mut commands: Commands,
) {
    for (button_ent, variant, disabled, pressed, hovered, bg_color, font_color) in q_buttons.iter()
    {
        set_button_colors(
            button_ent,
            variant,
            disabled,
            pressed,
            hovered.0,
            bg_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_button_styles_remove(
    q_buttons: Query<(
        Entity,
        &ButtonVariant,
        Has<InteractionDisabled>,
        Has<Pressed>,
        &Hovered,
        &ThemeBackgroundColor,
        &ThemeFontColor,
    )>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .for_each(|ent| {
            if let Ok((button_ent, variant, disabled, pressed, hovered, bg_color, font_color)) =
                q_buttons.get(ent)
            {
                set_button_colors(
                    button_ent,
                    variant,
                    disabled,
                    pressed,
                    hovered.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_button_colors(
    button_ent: Entity,
    variant: &ButtonVariant,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    bg_color: &ThemeBackgroundColor,
    font_color: &ThemeFontColor,
    commands: &mut Commands,
) {
    let bg_token = match (variant, disabled, pressed, hovered) {
        (ButtonVariant::Normal, true, _, _) => tokens::BUTTON_BG_DISABLED,
        (ButtonVariant::Normal, false, true, _) => tokens::BUTTON_BG_PRESSED,
        (ButtonVariant::Normal, false, false, true) => tokens::BUTTON_BG_HOVER,
        (ButtonVariant::Normal, false, false, false) => tokens::BUTTON_BG,
        (ButtonVariant::Primary, true, _, _) => tokens::BUTTON_PRIMARY_BG_DISABLED,
        (ButtonVariant::Primary, false, true, _) => tokens::BUTTON_PRIMARY_BG_PRESSED,
        (ButtonVariant::Primary, false, false, true) => tokens::BUTTON_PRIMARY_BG_HOVER,
        (ButtonVariant::Primary, false, false, false) => tokens::BUTTON_PRIMARY_BG,
        (ButtonVariant::Selected, true, _, _) => tokens::BUTTON_SELECTED_BG_DISABLED,
        (ButtonVariant::Selected, false, true, _) => tokens::BUTTON_SELECTED_BG_PRESSED,
        (ButtonVariant::Selected, false, false, true) => tokens::BUTTON_SELECTED_BG_HOVER,
        (ButtonVariant::Selected, false, false, false) => tokens::BUTTON_SELECTED_BG,
        (ButtonVariant::Plain, true, _, _) => tokens::BUTTON_PLAIN_BG_DISABLED,
        (ButtonVariant::Plain, false, true, _) => tokens::BUTTON_PLAIN_BG_PRESSED,
        (ButtonVariant::Plain, false, false, true) => tokens::BUTTON_PLAIN_BG_HOVER,
        (ButtonVariant::Plain, false, false, false) => tokens::BUTTON_PLAIN_BG,
    };

    let font_color_token = match (variant, disabled) {
        (ButtonVariant::Normal | ButtonVariant::Selected | ButtonVariant::Plain, true) => {
            tokens::BUTTON_TEXT_DISABLED
        }
        (ButtonVariant::Normal | ButtonVariant::Selected | ButtonVariant::Plain, false) => {
            tokens::BUTTON_TEXT
        }
        (ButtonVariant::Primary, true) => tokens::BUTTON_PRIMARY_TEXT_DISABLED,
        (ButtonVariant::Primary, false) => tokens::BUTTON_PRIMARY_TEXT,
    };

    // Change background color
    if bg_color.0 != bg_token {
        commands
            .entity(button_ent)
            .insert(ThemeBackgroundColor(bg_token));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(button_ent)
            .insert(ThemeFontColor(font_color_token));
    }
}

/// Plugin which registers the systems for updating the button styles.
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_button_styles, update_button_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
