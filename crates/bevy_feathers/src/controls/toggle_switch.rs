use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PreUpdate};
use bevy_core_widgets::{Callback, CoreCheckbox, ValueChange};
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
    spawn::SpawnRelated,
    system::{Commands, In, Query},
    world::Mut,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{BorderRadius, Checked, InteractionDisabled, Node, PositionType, UiRect, Val};

use crate::{
    constants::size,
    cursor::EntityCursor,
    theme::{ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// Parameters for the toggle switch template, passed to [`toggle_switch`] function.
#[derive(Default)]
pub struct ToggleSwitchProps {
    /// Change handler
    pub on_change: Callback<In<ValueChange<bool>>>,
}

/// Marker for the toggle switch outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchOutline;

/// Marker for the toggle switch slide
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchSlide;

/// Template function to spawn a toggle switch.
///
/// # Arguments
/// * `props` - construction properties for the toggle switch.
/// * `overrides` - a bundle of components that are merged in with the normal toggle switch components.
pub fn toggle_switch<B: Bundle>(props: ToggleSwitchProps, overrides: B) -> impl Bundle {
    (
        Node {
            width: size::TOGGLE_WIDTH,
            height: size::TOGGLE_HEIGHT,
            border: UiRect::all(Val::Px(2.0)),
            ..Default::default()
        },
        CoreCheckbox {
            on_change: props.on_change,
        },
        ToggleSwitchOutline,
        BorderRadius::all(Val::Px(5.0)),
        ThemeBackgroundColor(tokens::SWITCH_BG),
        ThemeBorderColor(tokens::SWITCH_BORDER),
        AccessibilityNode(accesskit::Node::new(Role::Switch)),
        Hovered::default(),
        EntityCursor::System(bevy_window::SystemCursorIcon::Pointer),
        TabIndex(0),
        overrides,
        children![(
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                width: Val::Percent(50.),
                ..Default::default()
            },
            BorderRadius::all(Val::Px(3.0)),
            ToggleSwitchSlide,
            ThemeBackgroundColor(tokens::SWITCH_SLIDE),
        )],
    )
}

fn update_switch_styles(
    q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeBorderColor,
        ),
        (
            With<ToggleSwitchOutline>,
            Or<(Changed<Hovered>, Added<Checked>, Added<InteractionDisabled>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_slide: Query<(&mut Node, &ThemeBackgroundColor), With<ToggleSwitchSlide>>,
    mut commands: Commands,
) {
    for (switch_ent, disabled, checked, hovered, outline_bg, outline_border) in q_switches.iter() {
        let Some(slide_ent) = q_children
            .iter_descendants(switch_ent)
            .find(|en| q_slide.contains(*en))
        else {
            continue;
        };
        // Safety: since we just checked the query, should always work.
        let (ref mut slide_style, slide_color) = q_slide.get_mut(slide_ent).unwrap();
        set_switch_colors(
            switch_ent,
            slide_ent,
            disabled,
            checked,
            hovered.0,
            outline_bg,
            outline_border,
            slide_style,
            slide_color,
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
            &Hovered,
            &ThemeBackgroundColor,
            &ThemeBorderColor,
        ),
        With<ToggleSwitchOutline>,
    >,
    q_children: Query<&Children>,
    mut q_slide: Query<(&mut Node, &ThemeBackgroundColor), With<ToggleSwitchSlide>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((switch_ent, disabled, checked, hovered, outline_bg, outline_border)) =
                q_switches.get(ent)
            {
                let Some(slide_ent) = q_children
                    .iter_descendants(switch_ent)
                    .find(|en| q_slide.contains(*en))
                else {
                    return;
                };
                // Safety: since we just checked the query, should always work.
                let (ref mut slide_style, slide_color) = q_slide.get_mut(slide_ent).unwrap();
                set_switch_colors(
                    switch_ent,
                    slide_ent,
                    disabled,
                    checked,
                    hovered.0,
                    outline_bg,
                    outline_border,
                    slide_style,
                    slide_color,
                    &mut commands,
                );
            }
        });
}

fn set_switch_colors(
    switch_ent: Entity,
    slide_ent: Entity,
    disabled: bool,
    checked: bool,
    hovered: bool,
    outline_bg: &ThemeBackgroundColor,
    outline_border: &ThemeBorderColor,
    slide_style: &mut Mut<Node>,
    slide_color: &ThemeBackgroundColor,
    commands: &mut Commands,
) {
    let outline_border_token = match (disabled, hovered) {
        (true, _) => tokens::SWITCH_BORDER_DISABLED,
        (false, true) => tokens::SWITCH_BORDER_HOVER,
        _ => tokens::SWITCH_BORDER,
    };

    let outline_bg_token = match (disabled, checked) {
        (true, true) => tokens::SWITCH_BG_CHECKED_DISABLED,
        (true, false) => tokens::SWITCH_BG_DISABLED,
        (false, true) => tokens::SWITCH_BG_CHECKED,
        (false, false) => tokens::SWITCH_BG,
    };

    let slide_token = match disabled {
        true => tokens::SWITCH_SLIDE_DISABLED,
        false => tokens::SWITCH_SLIDE,
    };

    let slide_pos = match checked {
        true => Val::Percent(50.),
        false => Val::Percent(0.),
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

    // Change slide color
    if slide_color.0 != slide_token {
        commands
            .entity(slide_ent)
            .insert(ThemeBackgroundColor(slide_token));
    }

    // Change slide position
    if slide_pos != slide_style.left {
        slide_style.left = slide_pos;
    }
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
