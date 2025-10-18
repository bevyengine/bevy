use bevy_derive::Deref;
use bevy_ecs::{prelude::*, relationship::Relationship};

use crate::{ComputedTextBlock, TextEntities, TextLayoutInfo};

#[derive(Component, Debug, PartialEq, Eq, Deref)]
#[relationship_target(relationship = TextOutput, linked_spawn)]
/// Root text element
pub struct TextRoot(Entity);

impl TextRoot {
    /// getter
    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Default, Copy, Clone)]
/// Text marker component
pub struct TextSection;

#[derive(Component, Debug, PartialEq, Eq, Deref)]
#[relationship(relationship_target = TextRoot)]
#[require(TextLayoutInfo, ComputedTextBlock, TextEntities)]
/// Output text element
pub struct TextOutput(pub Entity);

/// Output target id
#[derive(Component, Debug, PartialEq, Deref)]
pub struct TextTarget(Entity);

impl Default for TextTarget {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// Update text roots
pub fn update_text_roots_system(
    mut commands: Commands,
    orphan_query: Query<Entity, (With<TextSection>, Without<ChildOf>, Without<TextRoot>)>,
    child_query: Query<(Entity, &ChildOf, Has<TextRoot>), With<TextSection>>,
    parent_query: Query<&TextSection>,
) {
    for orphan_id in orphan_query.iter() {
        commands.spawn(TextOutput(orphan_id));
    }

    for (child_id, child_of, has_root) in child_query.iter() {
        let parent_is_text = parent_query.contains(child_of.get());
        if parent_is_text && has_root {
            // entity is not a root
            commands.entity(child_id).remove::<TextRoot>();
        } else if !parent_is_text && !has_root {
            // Root entity is not already a root
            commands.spawn(TextOutput(child_id));
        }
    }
}

/// update text entities lists
pub fn update_text_entities_system(
    mut buffer: Local<Vec<Entity>>,
    mut entities_query: Query<(Entity, &mut TextEntities, &TextOutput)>,
    mut targets_query: Query<&mut TextTarget>,
    children_query: Query<&Children, With<TextSection>>,
) {
    for (output_entity, mut entities, layout) in entities_query.iter_mut() {
        buffer.push(layout.get());
        for entity in children_query.iter_descendants_depth_first(layout.get()) {
            buffer.push(entity);
        }
        if buffer.as_slice() != entities.0.as_slice() {
            entities.0.clear();
            entities.0.extend_from_slice(&buffer);

            let mut targets_iter = targets_query.iter_many_mut(entities.0.as_slice());
            while let Some(mut target) = targets_iter.fetch_next() {
                target.0 = output_entity;
            }
        }
        buffer.clear();
    }
}

/// detect changes
pub fn detect_text_needs_rerender<T: Component>(
    text_query: Query<&TextTarget, Or<(Changed<T>, Changed<crate::TextFont>)>>,
    mut output_query: Query<&mut ComputedTextBlock>,
) {
    for target in text_query.iter() {
        if let Ok(mut computed) = output_query.get_mut(target.0) {
            computed.needs_rerender = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, Update};

    #[derive(Component)]
    #[require(TextTarget, TextSection)]
    struct Text;
    #[test]
    pub fn test_identify_text_roots() {
        let mut app = App::new();

        app.add_systems(Update, update_text_roots_system);

        let world = app.world_mut();

        let root_id = world.spawn(Text).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TextRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TextOutput)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);
    }

    #[test]
    pub fn test_despawn_text_layout_on_despawn_text_root() {
        let mut app = App::new();

        app.add_systems(Update, update_text_roots_system);

        let world = app.world_mut();

        let root_id = world.spawn(Text).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TextRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TextOutput)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);

        world.despawn(root_id);

        assert_eq!(world.query::<&TextOutput>().iter(world).count(), 0);
    }

    #[test]
    pub fn test_text_children_arent_roots() {
        let mut app = App::new();

        app.add_systems(Update, update_text_roots_system);

        let world = app.world_mut();

        let root_id = world.spawn((Text, children![Text, Text])).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TextRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TextOutput)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);
    }

    #[test]
    pub fn test_text_entity_with_non_text_parent_is_a_root() {
        let mut app = App::new();

        app.add_systems(Update, update_text_roots_system);

        let world = app.world_mut();

        let non_text_parent_id = world
            .spawn((children![(Text, children![Text, Text],)],))
            .id();

        app.update();

        let world = app.world_mut();

        let (root_id, _, root) = world
            .query::<(Entity, &Text, &TextRoot)>()
            .single(world)
            .unwrap();

        assert_ne!(non_text_parent_id, root_id);

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TextOutput)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);
    }

    #[test]
    pub fn test_a_root_that_gains_a_text_parent_is_no_longer_a_root() {
        let mut app = App::new();

        app.add_systems(Update, update_text_roots_system);

        let world = app.world_mut();

        let root_id = world.spawn(Text).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TextRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TextOutput)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);

        let parent_id = world.spawn(Text).add_child(root_id).id();

        app.update();
        let world = app.world_mut();

        let (new_root_id, _, root) = world
            .query::<(Entity, &Text, &TextRoot)>()
            .single(world)
            .unwrap();

        let root_layout_id = root.0;

        assert_eq!(new_root_id, parent_id);
        assert_ne!(root_layout_id, layout_id);

        let (new_layout_id, new_layout) = world
            .query::<(Entity, &TextOutput)>()
            .single(world)
            .unwrap();

        assert_eq!(new_layout_id, root_layout_id);
        assert_eq!(new_layout.0, new_root_id);
    }

    #[test]
    pub fn test_text_entities_contains_root() {
        let mut app = App::new();

        app.add_systems(
            Update,
            (update_text_roots_system, update_text_entities_system).chain(),
        );

        let world = app.world_mut();

        let id = world.spawn(Text).id();

        app.update();

        let world = app.world_mut();

        let entities = world.query::<&TextEntities>().single(world).unwrap();

        assert_eq!(entities.len(), 1);
        assert_eq!(entities.0[0], id);
    }

    #[test]
    pub fn test_number_of_texts_and_length_of_text_entities_is_equal() {
        let mut app = App::new();

        app.add_systems(
            Update,
            (update_text_roots_system, update_text_entities_system)
                .chain()
                .chain(),
        );

        let world = app.world_mut();
        world.spawn((
            Text,
            children![Text, (Text, children![Text, Text]), Text, Text],
        ));

        app.update();

        let world = app.world_mut();
        let sections = world.query::<&TextEntities>().single(world).unwrap();
        assert_eq!(sections.len(), 7);

        app.update();

        let world = app.world_mut();
        let sections = world.query::<&TextEntities>().single(world).unwrap();
        assert_eq!(sections.len(), 7);
    }
}
