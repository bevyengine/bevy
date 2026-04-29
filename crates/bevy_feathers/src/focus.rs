//! This module contains the infrastructure needed for displaying focus outlines.
use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::With,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_input_focus::{InputFocus, InputFocusVisible};
use bevy_platform::collections::HashSet;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{Outline, UiSystems, Val};

use crate::{theme::UiTheme, tokens};

/// A marker component which indicates that this entity should display a visible focus outline
/// when either it, or its ancestor, are focused. Insert this into a widget on the entity that
/// you wish to display a focus outline.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FocusIndicator;

/// A marker component which indicates that this entity should display a visible focus outline
/// when either it, or any descendant, are focused. Insert this into a widget on the entity that
/// you wish to display a focus outline.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FocusWithinIndicator;

fn manage_focus_indicators(
    mut commands: Commands,
    input_focus: Res<InputFocus>,
    input_focus_visible: Res<InputFocusVisible>,
    q_indicators: Query<Entity, With<FocusIndicator>>,
    q_within_indicators: Query<Entity, With<FocusWithinIndicator>>,
    q_children: Query<&Children>,
    q_parents: Query<&ChildOf>,
    theme: Res<UiTheme>,
) {
    if !input_focus.is_changed() && !input_focus_visible.is_changed() && !theme.is_changed() {
        return;
    }

    let mut visited = HashSet::<Entity>::with_capacity(q_indicators.count());
    if let Some(focus) = input_focus.get()
        && input_focus_visible.0
    {
        // Look for focus in descendants
        for entity in q_children
            .iter_descendants(focus)
            .chain(core::iter::once(focus))
        {
            if q_indicators.contains(entity) {
                commands.entity(entity).insert(Outline {
                    color: theme.color(&tokens::FOCUS_RING),
                    width: Val::Px(2.0),
                    offset: Val::Px(2.0),
                });
                visited.insert(entity);
            }
        }

        // Look for focus in ancestors
        for entity in q_parents
            .iter_ancestors(focus)
            .chain(core::iter::once(focus))
        {
            if q_within_indicators.contains(entity) {
                commands.entity(entity).insert(Outline {
                    color: theme.color(&tokens::FOCUS_RING),
                    width: Val::Px(2.0),
                    offset: Val::Px(2.0),
                });
                visited.insert(entity);
            }
        }
    }

    for entity in q_indicators.iter().chain(q_within_indicators.iter()) {
        if !visited.contains(&entity) {
            commands.entity(entity).remove::<Outline>();
        }
    }
}

/// Plugin which registers the systems for updating focus outlines.
pub struct FocusOutlinesPlugin;

impl Plugin for FocusOutlinesPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            manage_focus_indicators.in_set(UiSystems::Content),
        );
    }
}
