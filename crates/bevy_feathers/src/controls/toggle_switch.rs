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
    system::{Commands, In, Query, Res},
    world::Mut,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_math::curve::EaseFunction;
use bevy_picking::{hover::Hovered, PickingSystems};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{BorderRadius, Checked, InteractionDisabled, Node, PositionType, UiRect, Val};

use crate::{
    constants::size,
    cursor::EntityCursor,
    palette,
    theme::{ThemeBackgroundColor, ThemeBorderColor, UiTheme},
    tokens,
    transition::{AnimatedTransition, BackgroundColorTransition, LeftPercentTransition},
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
        ThemeBorderColor(tokens::SWITCH_BORDER),
        AnimatedTransition::<BackgroundColorTransition>::new(
            palette::GRAY_0.to_srgba(),
            palette::GRAY_1.to_srgba(),
        )
        .with_duration(0.25)
        .with_ease(EaseFunction::Linear),
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
            AnimatedTransition::<LeftPercentTransition>::new(0., 50.)
                .with_duration(0.25)
                .with_ease(EaseFunction::CubicInOut),
            ThemeBackgroundColor(tokens::SWITCH_SLIDE),
        )],
    )
}

fn update_switch_styles(
    mut q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &Hovered,
            &mut AnimatedTransition<BackgroundColorTransition>,
            &ThemeBorderColor,
        ),
        (
            With<ToggleSwitchOutline>,
            Or<(Changed<Hovered>, Added<Checked>, Added<InteractionDisabled>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_slide: Query<
        (
            &mut AnimatedTransition<LeftPercentTransition>,
            &ThemeBackgroundColor,
        ),
        With<ToggleSwitchSlide>,
    >,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    for (switch_ent, disabled, checked, hovered, mut outline_bg, outline_border) in
        q_switches.iter_mut()
    {
        let Some(slide_ent) = q_children
            .iter_descendants(switch_ent)
            .find(|en| q_slide.contains(*en))
        else {
            continue;
        };
        let (ref mut slide_transition, slide_color) = q_slide.get_mut(slide_ent).unwrap();
        set_switch_colors(
            switch_ent,
            slide_ent,
            disabled,
            checked,
            hovered.0,
            outline_bg.as_mut(),
            outline_border,
            slide_transition,
            slide_color,
            &theme,
            &mut commands,
        );
    }
}

fn update_switch_styles_remove(
    mut q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &Hovered,
            &mut AnimatedTransition<BackgroundColorTransition>,
            &ThemeBorderColor,
        ),
        With<ToggleSwitchOutline>,
    >,
    q_children: Query<&Children>,
    mut q_slide: Query<
        (
            &mut AnimatedTransition<LeftPercentTransition>,
            &ThemeBackgroundColor,
        ),
        With<ToggleSwitchSlide>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((switch_ent, disabled, checked, hovered, mut outline_bg, outline_border)) =
                q_switches.get_mut(ent)
            {
                let Some(slide_ent) = q_children
                    .iter_descendants(switch_ent)
                    .find(|en| q_slide.contains(*en))
                else {
                    return;
                };
                let (ref mut slide_transition, slide_color) = q_slide.get_mut(slide_ent).unwrap();
                set_switch_colors(
                    switch_ent,
                    slide_ent,
                    disabled,
                    checked,
                    hovered.0,
                    outline_bg.as_mut(),
                    outline_border,
                    slide_transition,
                    slide_color,
                    &theme,
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
    outline_bg: &mut AnimatedTransition<BackgroundColorTransition>,
    outline_border: &ThemeBorderColor,
    slide_transition: &mut Mut<AnimatedTransition<LeftPercentTransition>>,
    slide_color: &ThemeBackgroundColor,
    theme: &Res<'_, UiTheme>,
    commands: &mut Commands,
) {
    let outline_border_token = match (disabled, hovered) {
        (true, _) => tokens::SWITCH_BORDER_DISABLED,
        (false, true) => tokens::SWITCH_BORDER_HOVER,
        _ => tokens::SWITCH_BORDER,
    };

    match disabled {
        true => outline_bg.set_values(
            theme.color(tokens::SWITCH_BG_DISABLED).to_srgba(),
            theme.color(tokens::SWITCH_BG_CHECKED_DISABLED).to_srgba(),
        ),
        false => outline_bg.set_values(
            theme.color(tokens::SWITCH_BG).to_srgba(),
            theme.color(tokens::SWITCH_BG_CHECKED).to_srgba(),
        ),
    };

    let slide_token = match disabled {
        true => tokens::SWITCH_SLIDE_DISABLED,
        false => tokens::SWITCH_SLIDE,
    };

    match checked {
        true => {
            slide_transition.start();
            outline_bg.start();
        }
        false => {
            slide_transition.reverse();
            outline_bg.reverse();
        }
    };

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
