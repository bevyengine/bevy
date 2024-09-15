use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, With},
    system::{Commands, Local, Query},
};
use bevy_hierarchy::Children;
use bevy_reflect::prelude::*;

use crate::Node;

const ALPHA_ROUNDING_ERROR: f32 = 0.0000001;

fn recursively_propagate_opacity_value(
    mut first_traversal: bool,
    mut accumulated_opacity: f32,
    seen_propagators: &mut EntityHashSet,
    c: &mut Commands,
    children_query: &Query<&Children>,
    nodes: &mut Query<
        (
            Option<&PropagateOpacity>,
            Option<&mut OpacityModifier>,
            Has<BlockOpacityPropagation>,
        ),
        With<Node>,
    >,
    entity: Entity,
) {
    let Ok((maybe_propagator, maybe_modifier, has_block_propagation)) = nodes.get_mut(entity)
    else {
        return;
    };

    // Apply the block.
    if has_block_propagation {
        accumulated_opacity = 1.0;
    }

    // Handle the case that this node has `PropagateOpacity`.
    if let Some(PropagateOpacity(value)) = maybe_propagator {
        // Track seen.
        if !seen_propagators.insert(entity) {
            // If we've already seen this propagator, then this node and its children must have already
            // been updated once, so we don't want to overwrite the restoration values.
            first_traversal = false;
        }

        // Accumulate this value.
        // - Ignoring 1.0 hopefully avoids weird floating point issues that would invalidate the 1.0 check down
        //   below.
        if !value.is_nan() && *value != 1.0 {
            accumulated_opacity *= *value;
        }
    }

    // No need to continue if opacity won't be changed.
    // - Don't exit if not the first traversal in case somehow we went from non-1.0 accumulated to 1.0
    //   accumulated by adding in ancestor opacities (e.g. if this node propagates 0.5 and an ancestor
    //   propagates 2.0).
    if first_traversal && (accumulated_opacity - 1.0).abs() <= ALPHA_ROUNDING_ERROR {
        return;
    }

    // Update the entity's cached modifier, for use in rendering.
    let new_modifier = OpacityModifier(accumulated_opacity);

    if let Some(mut modifier) = maybe_modifier {
        *modifier = new_modifier;
    } else {
        c.entity(entity).insert(new_modifier);
    }

    // Iterate into children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        recursively_propagate_opacity_value(
            first_traversal,
            accumulated_opacity,
            seen_propagators,
            c,
            children_query,
            nodes,
            *child,
        );
    }
}

/// Applies all opacity modifiers throughout the hierarchy.
pub(crate) fn propagate_opacity_values(
    // Optimization to reduce redundant traversals by 50%.
    mut seen_propagators: Local<EntityHashSet>,
    mut c: Commands,
    propagators: Query<Entity, With<PropagateOpacity>>,
    children: Query<&Children>,
    mut nodes: Query<
        (
            Option<&PropagateOpacity>,
            Option<&mut OpacityModifier>,
            Has<BlockOpacityPropagation>,
        ),
        With<Node>,
    >,
) {
    seen_propagators.clear();

    for propagator in propagators.iter() {
        // Only do this in the base level so ancestor opacities properly reach all children.
        if seen_propagators.contains(&propagator) {
            continue;
        }

        recursively_propagate_opacity_value(
            true,
            1.0,
            &mut seen_propagators,
            &mut c,
            &children,
            &mut nodes,
            propagator,
        );
    }
}

/// Stores the aggregate opacity modifier that should be applied to UI components when rendering.
///
/// If an ancestor has [`PropagateOpacity`], then this component will be inserted to/updated on UI nodes automatically.
///
/// **IMPORTANT**: The modifier is **only** applied for each tick where it is mutated. If you are setting this manually,
/// then you need to trigger change detection on it every tick.
#[derive(Component, Copy, Clone, Deref, DerefMut, Debug, Default)]
pub struct OpacityModifier(pub f32);

/// Marker component that prevents ancestor [`PropagateOpacity`] values from affecting the entity or its children.
///
/// If the entity itself has [`PropagateOpacity`] in addition to `BlockOpacityPropagation`, then its
/// [`PropagateOpacity`] value will apply to itself and its children. Only ancestors are blocked.
#[derive(Component, Reflect, Copy, Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct BlockOpacityPropagation;

/// Component for setting an opacity multiplier on a hierarchy of nodes.
///
/// The propagated value will stack multiplicatively with other opacity multipliers in the same hierarchy.
/// Use [`BlockOpacityPropagation`] if you want to control how far a value can propagate.
///
/// This is a convenient tool for fading in/fading out pop-ups like on-hover help text. However, it may not be
/// efficient to *hide* those popups using inherited opacity, because it does require hierarchy traversal.
/// If perf becomes an issue, you should use [`Visibility::Hidden`](bevy_render::prelude::Visibility) to hide popups,
/// and only insert this component when animating a transition to full opacity.
#[derive(Component, Reflect, Default, Debug, Copy, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct PropagateOpacity(pub f32);
