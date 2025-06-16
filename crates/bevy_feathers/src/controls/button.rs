use bevy_app::{Plugin, PreUpdate};
use bevy_core_widgets::CoreButton;
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    query::With,
    schedule::IntoScheduleConfigs,
    system::{Query, SystemId},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_ui::{AlignItems, BorderRadius, JustifyContent, Node, Val};

/// Color variants for buttons.
#[derive(Default, Clone)]
pub enum ButtonVariant {
    /// The standard button appearance
    #[default]
    Normal,
    /// A button with a more prominent color, this is used for "call to action" buttons,
    /// default buttons for dialog boxes, and so on.
    Primary,
}

#[derive(Default, Clone)]
pub struct ButtonProps {
    pub variant: ButtonVariant,
    pub on_click: Option<SystemId>,
}

/// Q: How to pass in children?
/// Q: How to pass in theme?
/// Q: How to get asset handles?
/// Q: How to customize styles
pub fn button(props: ButtonProps) -> impl Bundle {
    (
        Node {
            // width: Val::Px(150.0),
            height: Val::Px(65.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        },
        CoreButton {
            on_click: props.on_click,
        },
        ButtonStyle,
        Hovered::default(),
        TabIndex(0),
        BorderRadius::all(Val::Px(4.0)),
        // BackgroundColor(NORMAL_BUTTON),
        // children![(
        //     Text::new("Button"),
        //     TextFont {
        //         font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        //         font_size: 33.0,
        //         ..default()
        //     },
        //     TextColor(Color::srgb(0.9, 0.9, 0.9)),
        //     TextShadow::default(),
        // )],
    )
}

/// Marker for styles to be applied to buttons.
#[derive(Component, Default, Clone)]
#[require(CoreButton, Hovered)]
struct ButtonStyle;

fn update_button_styles(q_buttons: Query<(), With<ButtonStyle>>) {}

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
