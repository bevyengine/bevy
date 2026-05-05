use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    world::Mut,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::prelude::*;
use bevy_ui::{
    percent, px, BorderRadius, Checked, InteractionDisabled, Node, PositionType, Pressed, UiRect,
};
use bevy_ui_widgets::{ActivateOnPress, Checkbox};

use crate::{
    constants::size,
    cursor::EntityCursor,
    focus::FocusIndicator,
    theme::{ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// A toggle switch widget.
///
/// This is spawnable by inheriting it as a "scene component".
///
/// # Emitted events
/// * [`bevy_ui_widgets::ValueChange<bool>`] with the new value when the toggle switch changes state.
///
/// These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the bundle
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersToggleSwitch;

impl FeathersToggleSwitch {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                width: size::TOGGLE_WIDTH,
                height: size::TOGGLE_HEIGHT,
                border: px(2),
                border_radius: px(5),
            }
            Checkbox
            FeathersToggleSwitch
            ThemeBackgroundColor(tokens::SWITCH_BG)
            ThemeBorderColor(tokens::SWITCH_BORDER)
            AccessibilityNode(accesskit::Node::new(Role::Switch))
            Hovered
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            TabIndex(0)
            FocusIndicator
            Children [(
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(0),
                    top: px(0),
                    bottom: px(0),
                    width: percent(50),
                    border: px(2),
                    border_radius: px(3),
                }
                ToggleSwitchSlide
                ThemeBackgroundColor(tokens::SWITCH_SLIDE_BG)
                ThemeBorderColor(tokens::SWITCH_SLIDE_BORDER)
            )]
        }
    }
}

/// Marker for the toggle switch slide
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchSlide;

/// Template function to spawn a toggle switch.
///
/// # Arguments
/// * `props` - construction properties for the toggle switch.
/// * `overrides` - a bundle of components that are merged in with the normal toggle switch components.
///
/// # Emitted events
/// * [`bevy_ui_widgets::ValueChange<bool>`] with the new value when the toggle switch changes state.
///
/// These events can be disabled by adding an [`bevy_ui::InteractionDisabled`] component to the bundle
#[deprecated(since = "0.19.0", note = "Use the toggle_switch() BSN function")]
pub fn toggle_switch_bundle<B: Bundle>(overrides: B) -> impl Bundle {
    (
        Node {
            width: size::TOGGLE_WIDTH,
            height: size::TOGGLE_HEIGHT,
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(5)),
            ..Default::default()
        },
        Checkbox,
        FeathersToggleSwitch,
        ThemeBackgroundColor(tokens::SWITCH_BG),
        ThemeBorderColor(tokens::SWITCH_BORDER),
        AccessibilityNode(accesskit::Node::new(Role::Switch)),
        Hovered::default(),
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer),
        TabIndex(0),
        FocusIndicator,
        overrides,
        children![(
            Node {
                position_type: PositionType::Absolute,
                left: percent(0),
                top: px(0),
                bottom: px(0),
                width: percent(50),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(3)),
                ..Default::default()
            },
            ToggleSwitchSlide,
            ThemeBackgroundColor(tokens::SWITCH_SLIDE_BG),
            ThemeBorderColor(tokens::SWITCH_SLIDE_BORDER)
        )],
    )
}

fn update_switch_styles(
    q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Pressed>,
            Has<ActivateOnPress>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeBorderColor,
        ),
        (
            With<FeathersToggleSwitch>,
            Or<(
                Changed<Hovered>,
                Added<Checked>,
                Added<Pressed>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_slide: Query<
        (&mut Node, &ThemeBackgroundColor, &ThemeBorderColor),
        With<ToggleSwitchSlide>,
    >,
    mut commands: Commands,
) {
    for (
        switch_ent,
        disabled,
        checked,
        pressed,
        activate_on_press,
        hovered,
        outline_bg,
        outline_border,
    ) in q_switches.iter()
    {
        let Some(slide_ent) = q_children
            .iter_descendants(switch_ent)
            .find(|en| q_slide.contains(*en))
        else {
            continue;
        };
        // Safety: since we just checked the query, should always work.
        let (ref mut slide_style, slide_bg_color, slide_border_color) =
            q_slide.get_mut(slide_ent).unwrap();
        set_switch_styles(
            switch_ent,
            slide_ent,
            disabled,
            checked,
            pressed,
            hovered.0,
            activate_on_press,
            outline_bg,
            outline_border,
            slide_style,
            slide_bg_color,
            slide_border_color,
            &mut commands,
        );
    }
}

fn update_switch_styles_remove(
    q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Pressed>,
            Has<ActivateOnPress>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeBorderColor,
        ),
        With<FeathersToggleSwitch>,
    >,
    q_children: Query<&Children>,
    mut q_slide: Query<
        (&mut Node, &ThemeBackgroundColor, &ThemeBorderColor),
        With<ToggleSwitchSlide>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut remove_pressed: RemovedComponents<Pressed>,
    mut remove_activate_on_press: RemovedComponents<ActivateOnPress>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .chain(remove_pressed.read())
        .chain(remove_activate_on_press.read())
        .for_each(|ent| {
            if let Ok((
                switch_ent,
                disabled,
                checked,
                pressed,
                activate_on_press,
                hovered,
                outline_bg,
                outline_border,
            )) = q_switches.get(ent)
            {
                let Some(slide_ent) = q_children
                    .iter_descendants(switch_ent)
                    .find(|en| q_slide.contains(*en))
                else {
                    return;
                };
                // Safety: since we just checked the query, should always work.
                let (ref mut slide_style, slide_bg_color, slide_border_color) =
                    q_slide.get_mut(slide_ent).unwrap();
                set_switch_styles(
                    switch_ent,
                    slide_ent,
                    disabled,
                    checked,
                    pressed,
                    hovered.0,
                    activate_on_press,
                    outline_bg,
                    outline_border,
                    slide_style,
                    slide_bg_color,
                    slide_border_color,
                    &mut commands,
                );
            }
        });
}

fn set_switch_styles(
    switch_ent: Entity,
    slide_ent: Entity,
    disabled: bool,
    checked: bool,
    pressed: bool,
    hovered: bool,
    activate_on_press: bool,
    outline_bg: &ThemeBackgroundColor,
    outline_border: &ThemeBorderColor,
    slide_style: &mut Mut<Node>,
    slide_bg_color: &ThemeBackgroundColor,
    slide_border_color: &ThemeBorderColor,
    commands: &mut Commands,
) {
    let outline_border_token = if checked {
        if disabled {
            tokens::SWITCH_BORDER_CHECKED_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_BORDER_CHECKED_PRESSED
        } else if hovered {
            tokens::SWITCH_BORDER_CHECKED_HOVER
        } else {
            tokens::SWITCH_BORDER_CHECKED
        }
    } else {
        if disabled {
            tokens::SWITCH_BORDER_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_BORDER_PRESSED
        } else if hovered {
            tokens::SWITCH_BORDER_HOVER
        } else {
            tokens::SWITCH_BORDER
        }
    };

    let outline_bg_token = if checked {
        if disabled {
            tokens::SWITCH_BG_CHECKED_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_BG_CHECKED_PRESSED
        } else if hovered {
            tokens::SWITCH_BG_CHECKED_HOVER
        } else {
            tokens::SWITCH_BG_CHECKED
        }
    } else {
        if disabled {
            tokens::SWITCH_BG_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_BG_PRESSED
        } else if hovered {
            tokens::SWITCH_BG_HOVER
        } else {
            tokens::SWITCH_BG
        }
    };

    let slide_border_token = if checked {
        if disabled {
            tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_SLIDE_BORDER_CHECKED_PRESSED
        } else if hovered {
            tokens::SWITCH_SLIDE_BORDER_CHECKED_HOVER
        } else {
            tokens::SWITCH_SLIDE_BORDER_CHECKED
        }
    } else {
        if disabled {
            tokens::SWITCH_SLIDE_BORDER_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_SLIDE_BORDER_PRESSED
        } else if hovered {
            tokens::SWITCH_SLIDE_BORDER_HOVER
        } else {
            tokens::SWITCH_SLIDE_BORDER
        }
    };

    let slide_bg_token = if checked {
        if disabled {
            tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_SLIDE_BG_CHECKED_PRESSED
        } else if hovered {
            tokens::SWITCH_SLIDE_BG_CHECKED_HOVER
        } else {
            tokens::SWITCH_SLIDE_BG_CHECKED
        }
    } else {
        if disabled {
            tokens::SWITCH_SLIDE_BG_DISABLED
        } else if pressed && !activate_on_press {
            tokens::SWITCH_SLIDE_BG_PRESSED
        } else if hovered {
            tokens::SWITCH_SLIDE_BG_HOVER
        } else {
            tokens::SWITCH_SLIDE_BG
        }
    };

    let slide_pos = match checked {
        true => percent(50),
        false => percent(0),
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change outline background
    if outline_bg.0 != outline_bg_token {
        commands
            .entity(switch_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    // Change outline border
    if outline_border.0 != outline_border_token {
        commands
            .entity(switch_ent)
            .insert(ThemeBorderColor(outline_border_token));
    }

    // Change slide background color
    if slide_bg_color.0 != slide_bg_token {
        commands
            .entity(slide_ent)
            .insert(ThemeBackgroundColor(slide_bg_token));
    }

    // Change slide border color
    if slide_border_color.0 != slide_border_token {
        commands
            .entity(slide_ent)
            .insert(ThemeBorderColor(slide_border_token));
    }

    // Change slide position
    if slide_pos != slide_style.left {
        slide_style.left = slide_pos;
    }

    // Change cursor shape
    commands
        .entity(switch_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the toggle switch styles.
pub struct ToggleSwitchPlugin;

impl Plugin for ToggleSwitchPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_switch_styles, update_switch_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
