/// Helpers to create a basic pane using `bevy_feathers::containers::pane`.
/// Using these helpers requires the `bevy_feathers` feature to be enabled.
use bevy::{
    feathers::{
        constants::icons,
        containers::*,
        controls::*,
        display::{icon, label},
    },
    math::Rot2,
    prelude::*,
    ui::UiTransform,
    ui_widgets::Activate,
};

/// Marker component used to identify the icon that toggles pane visibility and rotation.
#[derive(Clone, Copy, Component, Default)]
struct PaneToggleIcon;

/// Marker component used to identify the collapsible pane body.
#[derive(Clone, Copy, Component, Default)]
pub struct PaneBody;

/// Creates a generic Feathers pane header with a toggle button.
pub fn feathers_pane_header(title: &str) -> impl Scene {
    bsn! {
        pane_header() Children [
            label(title),
            flex_spacer(),
            @FeathersToolButton {
                @variant: ButtonVariant::Plain,
            } Children [
                icon(icons::CHEVRON_DOWN) PaneToggleIcon
            ]
            on(toggle_pane_body)
        ]
    }
}

/// Toggles the pane body visibility and updates the toggle icon rotation.
fn toggle_pane_body(
    _event: On<Activate>,
    mut commands: Commands,
    mut pane_body_query: Query<&mut Node, With<PaneBody>>,
    icon_query: Query<Entity, With<PaneToggleIcon>>,
) {
    let Ok(mut node) = pane_body_query.single_mut() else {
        return;
    };

    let will_open = node.display == Display::None;

    node.display = if will_open {
        Display::Flex
    } else {
        Display::None
    };

    if let Ok(icon_entity) = icon_query.single() {
        let rotation = if will_open {
            Rot2::IDENTITY
        } else {
            Rot2::radians(std::f32::consts::FRAC_PI_2)
        };

        commands
            .entity(icon_entity)
            .insert(UiTransform::from_rotation(rotation));
    }
}
