use bevy_derive::Deref;
use bevy_ecs::{prelude::*, relationship::Relationship};

use crate::TextSpan;

#[derive(Component, Default)]
pub struct ComputedTextBlock(pub Vec<Entity>);

#[derive(Component, Default)]
/// Root text element
pub struct TextRoot;

/// Output target id
#[derive(Component, Debug, PartialEq, Deref)]
pub struct TextTarget(Entity);

impl Default for TextTarget {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// update text entities lists
pub fn update_text_entities_system(
    mut buffer: Local<Vec<Entity>>,
    mut root_query: Query<(Entity, &mut ComputedTextBlock, Option<&Children>), With<TextRoot>>,
    mut targets_query: Query<&mut TextTarget>,
    children_query: Query<&Children, With<TextSpan>>,
) {
    for (root_id, mut entities, maybe_children) in root_query.iter_mut() {
        buffer.push(root_id);
        if let Some(children) = maybe_children {
            for entity in children.iter() {
                buffer.push(entity);
                for entity in children_query.iter_descendants_depth_first(root_id) {
                    buffer.push(entity);
                }
            }
        }
        if buffer.as_slice() != entities.0.as_slice() {
            entities.0.clear();
            entities.0.extend_from_slice(&buffer);

            let mut targets_iter = targets_query.iter_many_mut(entities.0.iter().skip(1).copied());
            while let Some(mut target) = targets_iter.fetch_next() {
                target.0 = root_id;
            }
        }
        buffer.clear();
    }
}

pub fn update_roots(mut root_query: Query<(Entity, &mut TextTarget), With<TextRoot>>) {
    for (e, mut t) in root_query.iter_mut() {
        t.0 = e;
    }
}

/// detect changes
pub fn detect_text_needs_rerender<T: Component>(
    text_query: Query<&TextTarget, Or<(Changed<T>, Changed<crate::TextFont>)>>,
    mut output_query: Query<&mut ComputedTextBlock>,
) {
    for target in text_query.iter() {
        if let Ok(mut computed) = output_query.get_mut(target.0) {
            computed.set_changed();
        }
    }
}
