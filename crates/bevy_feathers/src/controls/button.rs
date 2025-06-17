use bevy_app::{Plugin, PreUpdate};
use bevy_core_widgets::CoreButton;
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    hierarchy::{ChildOf, Children},
    query::{Added, Changed, Has, Or, With},
    schedule::IntoScheduleConfigs,
    spawn::{SpawnRelated, SpawnableList},
    system::{Query, SystemId},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_ui::{
    AlignItems, BorderRadius, InteractionDisabled, JustifyContent, Node, Pressed, UiRect, Val,
};

use crate::{
    font_styles::InheritableFont,
    handle_or_path::HandleOrPath,
    theme::{self, ThemeBackgroundColor, ThemeFontColor},
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
            ..Default::default()
        },
        CoreButton {
            on_click: props.on_click,
        },
        props.variant,
        Hovered::default(),
        // Some(InteractionDisabled),
        TabIndex(0),
        BorderRadius::all(Val::Px(4.0)),
        ThemeBackgroundColor(theme::tokens::BUTTON_BG),
        ThemeFontColor(theme::tokens::BUTTON_TXT),
        InheritableFont {
            font: HandleOrPath::Path(
                // TODO: Need to have a font asset in the crate.
                // TODO: Need a copy of FiraSans-Medium
                "fonts/FiraSans-Bold.ttf".to_owned(),
            ),
            font_size: 16.0,
        },
        Children::spawn::<C>(props.children),
    )
}

fn update_button_styles(
    q_buttons: Query<
        (Has<InteractionDisabled>, Has<Pressed>, &Hovered),
        (
            With<ButtonVariant>,
            Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
        ),
    >,
) {
}

fn update_button_styles_remove() {}

pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_button_styles, update_button_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
