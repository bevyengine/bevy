use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    spawn::{SpawnRelated, SpawnableList},
    system::{Commands, Query},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{prelude::*, template_value};
use bevy_text::FontWeight;
use bevy_ui::{AlignItems, InteractionDisabled, JustifyContent, Node, Pressed, UiRect, Val};
use bevy_ui_widgets::Button;

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    focus::FocusIndicator,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeFontColor},
    tokens,
};

/// Color variants for buttons. This also functions as a component used by the dynamic styling
/// system to identify which entities are buttons.
#[derive(Component, Default, Clone, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component, Clone, Default)]
pub enum ButtonVariant {
    /// The standard button appearance
    #[default]
    Normal,
    /// A button with a more prominent color, this is used for "call to action" buttons,
    /// default buttons for dialog boxes, and so on.
    Primary,
    /// Don't display the button background unless hovering or pressed.
    Plain,
}

/// Parameters for the button template, passed to [`button`] function.
pub struct ButtonProps {
    /// Label for this button. This can contain multiple entities, which will be contained
    /// in a horizontal flexbox.
    pub caption: Box<dyn SceneList>,
    /// Color variant for the button.
    pub variant: ButtonVariant,
    /// Rounded corners options
    pub corners: RoundedCorners,
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
            variant: ButtonVariant::default(),
            corners: Default::default(),
        }
    }
}

/// Scene function to spawn a button.
///
/// # Arguments
/// * `props` - construction properties for the button.
///
/// # Emitted events
/// * [`bevy_ui_widgets::Activate`] when any of the following happens:
///     * the pointer is released while hovering over the button.
///     * the ENTER or SPACE key is pressed while the button has keyboard focus.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
pub fn button(props: ButtonProps) -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
            border_radius: {props.corners.to_border_radius(4.0)},
        }
        Button
        template_value(props.variant)
        Hovered
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
        TabIndex(0)
        FocusIndicator
        ThemeBackgroundColor(tokens::BUTTON_BG)
        ThemeFontColor(tokens::BUTTON_TEXT)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        Children [
            {props.caption}
        ]
    }
}

/// Tool button scene function: a smaller button for embedding in panel headers.
///
/// # Arguments
/// * `props` - construction properties for the button.
///
/// # Emitted events
/// * [`bevy_ui_widgets::Activate`] when any of the following happens:
///     * the pointer is released while hovering over the button.
///     * the ENTER or SPACE key is pressed while the button has keyboard focus.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
pub fn tool_button(props: ButtonProps) -> impl Scene {
    bsn! {
        :button(props)
        Node {
            padding: UiRect::axes(Val::Px(4.0), Val::Px(0.)),
            min_width: size::ROW_HEIGHT,
        }
    }
}

/// Parameters for the [`button_bundle`] template.
#[derive(Default)]
pub struct ButtonBundleProps {
    /// Color variant for the button.
    pub variant: ButtonVariant,
    /// Rounded corners options
    pub corners: RoundedCorners,
}

/// Template function to spawn a button.
///
/// # Arguments
/// * `props` - construction properties for the button.
///
/// # Emitted events
/// * [`bevy_ui_widgets::Activate`] when any of the following happens:
///     * the pointer is released while hovering over the button.
///     * the ENTER or SPACE key is pressed while the button has keyboard focus.
///
///  These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the entity
#[deprecated(since = "0.19.0", note = "Use the button() BSN function")]
pub fn button_bundle<C: SpawnableList<ChildOf> + Send + Sync + 'static, B: Bundle>(
    props: ButtonBundleProps,
    overrides: B,
    children: C,
) -> impl Bundle {
    (
        Node {
            height: size::ROW_HEIGHT,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
            flex_grow: 1.0,
            border_radius: props.corners.to_border_radius(4.0),
            ..Default::default()
        },
        Button,
        props.variant,
        Hovered::default(),
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer),
        TabIndex(0),
        FocusIndicator,
        ThemeBackgroundColor(tokens::BUTTON_BG),
        ThemeFontColor(tokens::BUTTON_TEXT),
        InheritableFont {
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
            ..Default::default()
        },
        overrides,
        Children::spawn(children),
    )
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
        Or<(
            Changed<Hovered>,
            Changed<ButtonVariant>,
            Added<Pressed>,
            Added<InteractionDisabled>,
        )>,
    >,
    mut commands: Commands,
) {
    for (button_ent, variant, disabled, pressed, hovered, bg_color, font_color) in q_buttons.iter()
    {
        set_button_styles(
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
                set_button_styles(
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

fn set_button_styles(
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
        (ButtonVariant::Plain, true, _, _) => tokens::BUTTON_PLAIN_BG_DISABLED,
        (ButtonVariant::Plain, false, true, _) => tokens::BUTTON_PLAIN_BG_PRESSED,
        (ButtonVariant::Plain, false, false, true) => tokens::BUTTON_PLAIN_BG_HOVER,
        (ButtonVariant::Plain, false, false, false) => tokens::BUTTON_PLAIN_BG,
    };

    let font_color_token = match (variant, disabled) {
        (ButtonVariant::Primary, true) => tokens::BUTTON_PRIMARY_TEXT_DISABLED,
        (ButtonVariant::Primary, false) => tokens::BUTTON_PRIMARY_TEXT,
        (ButtonVariant::Normal | ButtonVariant::Plain, true) => tokens::BUTTON_TEXT_DISABLED,
        (ButtonVariant::Normal | ButtonVariant::Plain, false) => tokens::BUTTON_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
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

    // Change cursor shape
    commands
        .entity(button_ent)
        .insert(EntityCursor::System(cursor_shape));
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
