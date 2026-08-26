use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_math::Rot2;
use bevy_picking::PickingSystems;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_scene::{bsn, Scene, SceneComponent};
use bevy_ui::{
    px, AlignItems, Checked, Display, InteractionDisabled, JustifyContent, Node, UiTransform,
};
use bevy_ui_widgets::Checkbox;
use bevy_window::SystemCursorIcon;

use crate::{
    constants::icons, cursor::EntityCursor, display::icon, focus::FocusIndicator,
    theme::InheritableThemeTextColor, tokens,
};

/// A toggle button which shows a chevron that points either right or down, used to expand or
/// collapse a panel. Functionally, this is equivalent to a checkbox, and has a [`Checked`]
/// state.
///
/// This is spawnable by inheriting it as a "scene component".
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersDisclosureToggle;

impl FeathersDisclosureToggle {
    fn scene() -> impl Scene {
        bsn!(
            Node {
                width: px(12),
                height: px(12),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
            }
            Checkbox
            EntityCursor::System(SystemCursorIcon::Pointer)
            FocusIndicator
            InheritableThemeTextColor(tokens::BUTTON_TEXT)
            TabIndex(0)
            Children [
                icon(icons::CHEVRON_RIGHT)
            ]
        )
    }
}

fn update_toggle_styles(
    mut q_toggle: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &mut UiTransform,
            &InheritableThemeTextColor,
        ),
        (
            With<FeathersDisclosureToggle>,
            Or<(Added<Checkbox>, Added<Checked>, Added<InteractionDisabled>)>,
        ),
    >,
    mut commands: Commands,
) {
    for (ent, disabled, checked, mut transform, text_color) in q_toggle.iter_mut() {
        set_toggle_styles(
            ent,
            disabled,
            checked,
            transform.as_mut(),
            text_color,
            &mut commands,
        );
    }
}

fn update_toggle_styles_remove(
    mut q_toggle: Query<
        (
            Has<InteractionDisabled>,
            Has<Checked>,
            &mut UiTransform,
            &InheritableThemeTextColor,
        ),
        With<FeathersDisclosureToggle>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((disabled, checked, mut transform, text_color)) = q_toggle.get_mut(ent) {
                set_toggle_styles(
                    ent,
                    disabled,
                    checked,
                    transform.as_mut(),
                    text_color,
                    &mut commands,
                );
            }
        });
}

fn set_toggle_styles(
    entity: Entity,
    disabled: bool,
    checked: bool,
    transform: &mut UiTransform,
    text_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    // It's effectively the same color as the caption of a "plain" variant tool button with an icon.
    let new_text_color_token = match disabled {
        true => tokens::BUTTON_TEXT_DISABLED,
        false => tokens::BUTTON_TEXT,
    };

    // Change icon color
    if new_text_color_token != text_color.0 {
        commands
            .entity(entity)
            .insert(InheritableThemeTextColor(new_text_color_token));
    }

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
