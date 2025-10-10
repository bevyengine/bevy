use bevy_derive::Deref;
use bevy_ecs::{prelude::*, relationship::Relationship};

use crate::TextEntities;

#[derive(Component, Debug, PartialEq, Deref)]
pub struct TextTarget(Entity);

impl Default for TextTarget {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// Update text roots
pub fn update_text_roots_system<T: Component, Root: RelationshipTarget, Layout: Relationship>(
    mut commands: Commands,
    orphan_query: Query<Entity, (With<T>, Without<ChildOf>, Without<Root>)>,
    child_query: Query<(Entity, &ChildOf, Has<Root>), With<T>>,
    parent_query: Query<&T>,
    non_text_root_query: Query<Entity, (With<Root>, Without<T>)>,
) {
    for orphan_id in orphan_query.iter() {
        commands.spawn(Layout::from(orphan_id));
    }

    for (child_id, child_of, has_root) in child_query.iter() {
        let parent_is_text = parent_query.contains(child_of.get());
        if parent_is_text && has_root {
            // entity is not a root
            commands.entity(child_id).remove::<Root>();
        } else if !parent_is_text && !has_root {
            // Root entity is not already a root
            commands.spawn(Layout::from(child_id));
        }
    }

    for id in non_text_root_query.iter() {
        commands.entity(id).remove::<Root>();
    }
}

pub fn update_text_entities<T: Component, Layout: Relationship>(
    mut buffer: Local<Vec<Entity>>,
    mut entities_query: Query<(&mut TextEntities, &Layout)>,
    children_query: Query<&Children, With<T>>,
) {
    for (mut entities, layout) in entities_query.iter_mut() {
        buffer.push(layout.get());
        for entity in children_query.iter_descendants_depth_first(layout.get()) {
            buffer.push(entity);
        }
        if buffer.as_slice() != entities.0.as_slice() {
            entities.0.clear();
            entities.0.extend_from_slice(&buffer);
        }
        buffer.clear();
    }
}

/// update text indices
pub fn detect_changed_text_() {}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, Update};
    use bevy_ecs::relationship::DescendantIter;

    #[derive(Component, Debug, PartialEq, Eq)]
    #[relationship_target(relationship = TestLayout, linked_spawn)]
    struct TestRoot(Entity);

    #[derive(Component, Debug, PartialEq, Eq)]
    #[relationship(relationship_target = TestRoot)]
    struct TestLayout(Entity);

    #[derive(Component)]
    #[require(TextTarget)]
    struct Text;
    #[test]
    pub fn test_identify_text_roots() {
        let mut app = App::new();

        app.add_systems(
            Update,
            update_text_roots_system::<Text, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(Text).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TestRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TestLayout)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);
    }

    #[test]
    pub fn test_despawn_text_layout_on_despawn_text_root() {
        let mut app = App::new();

        app.add_systems(
            Update,
            update_text_roots_system::<Text, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(Text).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TestRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TestLayout)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);

        world.despawn(root_id);

        assert_eq!(world.query::<&TestLayout>().iter(world).count(), 0);
    }

    #[test]
    pub fn test_text_children_arent_roots() {
        let mut app = App::new();

        app.add_systems(
            Update,
            update_text_roots_system::<Text, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn((Text, children![Text, Text])).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TestRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TestLayout)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);
    }

    #[test]
    pub fn test_text_entity_with_non_text_parent_is_a_root() {
        let mut app = App::new();

        app.add_systems(
            Update,
            update_text_roots_system::<Text, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let non_text_parent_id = world
            .spawn((children![(Text, children![Text, Text],)],))
            .id();

        app.update();

        let world = app.world_mut();

        let (root_id, _, root) = world
            .query::<(Entity, &Text, &TestRoot)>()
            .single(world)
            .unwrap();

        assert_ne!(non_text_parent_id, root_id);

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TestLayout)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);
    }

    #[test]
    pub fn test_a_root_that_gains_a_text_parent_is_no_longer_a_root() {
        let mut app = App::new();

        app.add_systems(
            Update,
            update_text_roots_system::<Text, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(Text).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world.query::<(&Text, &TestRoot)>().single(world).unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TestLayout)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);

        let parent_id = world.spawn(Text).add_child(root_id).id();

        app.update();
        let world = app.world_mut();

        let (new_root_id, _, root) = world
            .query::<(Entity, &Text, &TestRoot)>()
            .single(world)
            .unwrap();

        let root_layout_id = root.0;

        assert_eq!(new_root_id, parent_id);
        assert_ne!(root_layout_id, layout_id);

        let (new_layout_id, new_layout) = world
            .query::<(Entity, &TestLayout)>()
            .single(world)
            .unwrap();

        assert_eq!(new_layout_id, root_layout_id);
        assert_eq!(new_layout.0, new_root_id);
    }

    #[test]
    pub fn test_a_text_root_that_becomes_non_text_is_not_a_root() {
        let mut app = App::new();

        app.add_systems(
            Update,
            update_text_roots_system::<Text, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(Text).id();

        app.update();
        let world = app.world_mut();

        assert_eq!(world.query::<&TestRoot>().iter(world).count(), 1);
        assert_eq!(world.query::<&TestLayout>().iter(world).count(), 1);

        world.entity_mut(root_id).remove::<Text>();

        app.update();
        let world = app.world_mut();

        assert_eq!(world.query::<&TestRoot>().iter(world).count(), 0);
        assert_eq!(world.query::<&TestLayout>().iter(world).count(), 0);
    }
}
