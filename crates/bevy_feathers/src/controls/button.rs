use bevy_app::{Plugin, PreUpdate};
use bevy_core_widgets::CoreButton;
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or},
    schedule::IntoScheduleConfigs,
    spawn::{SpawnRelated, SpawnableList},
    system::{Commands, Query, SystemId},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_ui::{AlignItems, InteractionDisabled, JustifyContent, Node, Pressed, UiRect, Val};

use crate::{
    font_styles::InheritableFont,
    handle_or_path::HandleOrPath,
    theme::{self, corners::RoundedCorners, fonts, ThemeBackgroundColor, ThemeFontColor},
};

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
}

/// Parameters for the button template.
#[derive(Default, Clone)]
pub struct ButtonProps<C: SpawnableList<ChildOf>> {
    /// Color variant for the button.
    pub variant: ButtonVariant,
    /// Click handler
    pub on_click: Option<SystemId>,
    /// Used to specify the label or icond for the button.
    pub children: C,
}

/// Q: How to pass in children?
/// Q: How to pass in theme?
/// Q: How to get asset handles?
/// Q: How to customize styles
pub fn button<C: SpawnableList<ChildOf> + Send + Sync + 'static>(
    props: ButtonProps<C>,
) -> impl Bundle {
    (
        Node {
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.)),
            flex_grow: 1.0,
            ..Default::default()
        },
        CoreButton {
            on_click: props.on_click,
        },
        props.variant,
        Hovered::default(),
        // Some(InteractionDisabled),
        TabIndex(0),
        RoundedCorners::All.to_border_radius(4.0),
        ThemeBackgroundColor(theme::tokens::BUTTON_BG),
        ThemeFontColor(theme::tokens::BUTTON_TXT),
        InheritableFont {
            font: HandleOrPath::Path(fonts::REGULAR.to_owned()),
            font_size: 16.0,
        },
        Children::spawn::<C>(props.children),
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
        ),
        Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
    >,
    mut commands: Commands,
) {
    for (button_ent, variant, disabled, pressed, hovered, bg_color) in q_buttons.iter() {
        set_button_styles(
            button_ent,
            variant,
            disabled,
            pressed,
            hovered.0,
            bg_color,
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
    )>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .for_each(|ent| {
            if let Ok((button_ent, variant, disabled, pressed, hovered, bg_color)) =
                q_buttons.get(ent)
            {
                set_button_styles(
                    button_ent,
                    variant,
                    disabled,
                    pressed,
                    hovered.0,
                    bg_color,
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
    commands: &mut Commands,
) {
    let bg_token = match (variant, disabled, pressed, hovered) {
        (ButtonVariant::Normal, true, _, _) => theme::tokens::BUTTON_BG_DISABLED,
        (ButtonVariant::Normal, false, true, _) => theme::tokens::BUTTON_BG_PRESSED,
        (ButtonVariant::Normal, false, false, true) => theme::tokens::BUTTON_BG_HOVER,
        (ButtonVariant::Normal, false, false, false) => theme::tokens::BUTTON_BG,
        (ButtonVariant::Primary, true, _, _) => theme::tokens::BUTTON_PRIMARY_BG_DISABLED,
        (ButtonVariant::Primary, false, true, _) => theme::tokens::BUTTON_PRIMARY_BG_PRESSED,
        (ButtonVariant::Primary, false, false, true) => theme::tokens::BUTTON_PRIMARY_BG_HOVER,
        (ButtonVariant::Primary, false, false, false) => theme::tokens::BUTTON_PRIMARY_BG,
    };

    // Change background color
    if bg_color.0 != bg_token {
        commands
            .entity(button_ent)
            .insert(ThemeBackgroundColor(bg_token));
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
