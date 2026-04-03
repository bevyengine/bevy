//! This module contains the infrastructure needed for displaying focus outlines.
use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    query::With,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_input_focus::{InputFocus, InputFocusVisible};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{Outline, UiSystems, Val};

use crate::{theme::UiTheme, tokens};

/// A marker component which indicates that this entity should display a visible focus outline
/// when either it, or its ancestor, are focused. Insert this into a widget on the entity that
/// you wish to display a focus outline.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FocusIndicator;

fn manage_focus_indicators(
    mut commands: Commands,
    input_focus: Res<InputFocus>,
    input_focus_visible: Res<InputFocusVisible>,
    q_indicators: Query<Entity, With<FocusIndicator>>,
    q_ancestors: Query<&ChildOf>,
    theme: Res<UiTheme>,
) {
    if !input_focus.is_changed() && !input_focus_visible.is_changed() && !theme.is_changed() {
        return;
    }

    for entity in q_indicators.iter() {
        let is_focused = input_focus_visible.0
            && input_focus.0.is_some()
            && (Some(entity) == input_focus.0
                || q_ancestors
                    .iter_ancestors(entity)
                    .any(|ancestor| Some(ancestor) == input_focus.0));
        if !is_focused {
            commands.entity(entity).remove::<Outline>();
        } else {
            commands.entity(entity).insert(Outline {
                color: theme.color(&tokens::FOCUS_RING),
                width: Val::Px(2.0),
                offset: Val::Px(2.0),
            });
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
