//! This module contains the infrastructure needed for displaying focus outlines.

use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    query::With,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_input_focus::{InputFocus, InputFocusVisible};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{BorderRadius, GlobalZIndex, Node, Outline, PositionType, UiSystems, Val};

use crate::{theme::UiTheme, tokens};

/// A marker component which indicates that this entity should display a visible focus outline
/// when either it, or its ancestor, are focused. Insert this into a widget on the entity that
/// you wisth to display a focus outline.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FocusIndicator;

/// A marker used to identify a visible focus outline.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct FocusOutline;

fn focus_system(
    mut commands: Commands,
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    theme: Res<UiTheme>,
    q_focus_outlines: Query<Entity, With<FocusOutline>>,
    q_focus_anchors: Query<&BorderRadius, With<FocusIndicator>>,
    q_children: Query<&Children>,
) {
    if focus.is_changed() {
        // Start by despawning all the existing focus outlines
        for outline in q_focus_outlines.iter() {
            commands.entity(outline).despawn();
        }

        // Walk the descendants of the current focus element.
        if focus_visible.0
            && let Some(focused) = focus.0
        {
            spawn_focus_rings(
                &mut commands,
                focused,
                &q_children,
                &q_focus_anchors,
                theme.as_ref(),
            );
        }
    }
}

fn spawn_focus_rings(
    commands: &mut Commands,
    parent: Entity,
    q_children: &Query<&Children>,
    q_anchors: &Query<&BorderRadius, With<FocusIndicator>>,
    theme: &UiTheme,
) {
    if let Ok(radii) = q_anchors.get(parent) {
        let outline = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..Default::default()
                },
                FocusOutline,
                GlobalZIndex(100),
                Outline {
                    color: theme.color(tokens::FOCUS_RING),
                    width: Val::Px(2.0),
                    offset: Val::Px(2.0),
                },
                *radii,
            ))
            .id();
        commands.entity(parent).add_child(outline);
    } else if let Ok(children) = q_children.get(parent) {
        for child in children {
            spawn_focus_rings(commands, *child, q_children, q_anchors, theme);
        }
    }
}

/// Plugin which registers the systems for updating focus outlines.
pub struct FocusOutlinesPlugin;

impl Plugin for FocusOutlinesPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(PostUpdate, (focus_system).in_set(UiSystems::Content));
    }
}
