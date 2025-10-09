use crate::{Font, TextFont, TextLayoutInfo};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::prelude::*;
use bevy_utils::{default, once};
use cosmic_text::{Buffer, Metrics};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tracing::warn;

/// Text root
#[derive(Component, PartialEq)]
pub struct TextRoot(pub SmallVec<[Entity; 1]>);

/// Update text roots
pub fn update_text_roots<T: Component>(
    mut parents: Local<Vec<Entity>>,
    mut spans: Local<Vec<Entity>>,
    mut commands: Commands,
    mut text_node_query: Query<(
        Entity,
        Option<&ChildOf>,
        Option<&mut TextRoot>,
        Has<Children>,
        Ref<T>,
        Ref<TextFont>,
    )>,
    children_query: Query<(Option<&Children>, Ref<T>, Ref<TextFont>)>,
) {
    for (entity, maybe_child_of, maybe_text_root, has_children, text, style, font_size) in
        text_node_query.iter_mut()
    {
        if maybe_child_of.is_none_or(|parent| !children_query.contains(parent.get())) {
            // Either the text entity is an orphan, or its parent is not a text entity. It must be a root text entity.
            if has_children {
                parents.push(entity);
            } else {
                let new_text_root = TextRoot(smallvec::smallvec![entity]);
                if let Some(mut text_root) = maybe_text_root {
                    text_root.set_if_neq(new_text_root);
                    if text.is_changed() || style.is_changed() || font_size.is_changed() {
                        text_root.set_changed();
                    }
                } else {
                    commands.entity(entity).insert(new_text_root);
                }
            }
        } else if maybe_text_root.is_some() {
            // Not a root. Remove `TextRoot` component, if present.
            commands.entity(entity).remove::<TextRoot>();
        }
    }

    for root_entity in parents.drain(..) {
        spans.clear();
        let mut changed = false;

        fn walk_text_descendants<T: Component>(
            target: Entity,
            query: &Query<(
                Option<&Children>,
                Ref<T>,
                Ref<ComputedTextStyle>,
                Ref<ComputedFontSize>,
            )>,
            spans: &mut Vec<Entity>,
            changed: &mut bool,
        ) {
            spans.push(target);
            if let Ok((children, text, style, size)) = query.get(target) {
                *changed |= text.is_changed() || style.is_changed() || size.is_changed();
                if let Some(children) = children {
                    for child in children {
                        walk_text_descendants(*child, query, spans, changed);
                    }
                }
            }
        }

        walk_text_descendants(root_entity, &children_query, &mut spans, &mut changed);

        if let Ok((_, _, Some(mut text_root), ..)) = text_node_query.get_mut(root_entity) {
            if text_root.0.as_slice() != spans.as_slice() {
                text_root.0.clear();
                text_root.0.extend(spans.iter().copied());
            }
            if changed {
                text_root.set_changed();
            }
        } else {
            commands
                .entity(root_entity)
                .insert(TextRoot(SmallVec::from_slice(&spans)));
        }
    }
}
