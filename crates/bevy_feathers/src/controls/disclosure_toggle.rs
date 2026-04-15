use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Query, Res},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_math::Rot2;
use bevy_picking::PickingSystems;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, Scene};
use bevy_ui::{
    px, AlignItems, Checked, Display, InteractionDisabled, JustifyContent, Node, UiTransform,
};
use bevy_ui_widgets::Checkbox;
use bevy_window::SystemCursorIcon;

use crate::{
    constants::icons, cursor::EntityCursor, display::icon, focus::FocusIndicator, theme::UiTheme,
    tokens,
};

/// Marker for the disclosure toggle widget
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct DisclosureToggleStyle;

/// A toggle button which shows a chevron that points either right or down, used to expand or
/// collapse a panel. Functionally, this is equivalent to a checkbox, and has a [`Checked`]
/// state.
pub fn disclosure_toggle() -> impl Scene {
    bsn!(
        Node {
            width: px(12),
            height: px(12),
            display: Display::Flex,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Checkbox
        DisclosureToggleStyle
        EntityCursor::System(SystemCursorIcon::Pointer)
        FocusIndicator
        TabIndex(0)
        Children [
            :icon(icons::CHEVRON_RIGHT)
        ]
    )
}

fn update_toggle_styles(
    mut q_switches: Query<
        (Has<InteractionDisabled>, Has<Checked>, &mut UiTransform),
        (
            With<DisclosureToggleStyle>,
            Or<(Added<Checked>, Added<InteractionDisabled>)>,
        ),
    >,
    theme: Res<UiTheme>,
) {
    for (disabled, checked, mut transform) in q_switches.iter_mut() {
        set_toggle_colors(disabled, checked, transform.as_mut(), &theme);
    }
}

fn update_toggle_styles_remove(
    mut q_switches: Query<
        (Has<InteractionDisabled>, Has<Checked>, &mut UiTransform),
        With<DisclosureToggleStyle>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    theme: Res<UiTheme>,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((disabled, checked, mut transform)) = q_switches.get_mut(ent) {
                set_toggle_colors(disabled, checked, transform.as_mut(), &theme);
            }
        });
}

fn set_toggle_colors(
    disabled: bool,
    checked: bool,
    transform: &mut UiTransform,
    _theme: &Res<'_, UiTheme>,
) {
    let _slide_token = match disabled {
        true => tokens::SWITCH_SLIDE_DISABLED,
        false => tokens::SWITCH_SLIDE,
    };

    match checked {
        true => {
            transform.rotation = Rot2::turn_fraction(0.25);
        }
        false => {
            transform.rotation = Rot2::turn_fraction(0.0);
        }
    };
}

/// Plugin which registers the systems for updating the toggle switch styles.
pub struct DisclosureTogglePlugin;

impl Plugin for DisclosureTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (update_toggle_styles, update_toggle_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
