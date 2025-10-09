use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, relationship::Relationship};

use smallvec::SmallVec;

/// Index of a text entity in a text layout
#[derive(Component, Debug, PartialEq, Deref, Default)]
pub struct TextIndex(usize);
#[derive(Component, Debug, PartialEq, Deref, DerefMut, Default)]
pub struct TextSections(pub SmallVec<[Entity; 1]>);

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

/// update text indices
pub fn update_text_indices_system<Root: RelationshipTarget>(
    mut sections: Local<Vec<Entity>>,
    root_query: Query<(Entity, &Root), With<Root>>,
    descendants: Query<&Children, With<TextIndex>>,
    mut text_index_query: Query<(&mut TextIndex, &mut TextTarget)>,
    mut text_target_query: Query<&mut TextSections>,
) {
    for (root_id, root) in root_query.iter() {
        let layout_id = root.iter().next().unwrap();
        sections.clear();
        sections.push(root_id);

        let (mut index, mut target) = text_index_query.get_mut(root_id).ok().unwrap();
        index.set_if_neq(TextIndex(0));
        target.set_if_neq(TextTarget(layout_id));

        sections.clear();
        for (i, text_id) in descendants.iter_descendants(root_id).enumerate() {
            sections.push(text_id);
            let (mut index, mut target) = text_index_query.get_mut(text_id).ok().unwrap();
            index.set_if_neq(TextIndex(i + 1));
            target.set_if_neq(TextTarget(layout_id));
        }

        let mut text_sections = text_target_query.get_mut(layout_id).ok().unwrap();
        if text_sections.as_slice() != sections.as_slice() {
            text_sections.clear();
            text_sections.extend(sections.iter().copied());
        }
    }
}

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
    #[require(TextSections)]
    struct TestLayout(Entity);

    #[derive(Component)]
    #[require(TextIndex, TextTarget)]
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

    #[test]
    pub fn test_text_root_has_index_0() {
        let mut app = App::new();

        app.add_systems(
            Update,
            (
                update_text_roots_system::<Text, TestRoot, TestLayout>,
                update_text_indices_system::<TestRoot>,
            )
                .chain(),
        );

        let world = app.world_mut();

        world.spawn(Text);

        app.update();

        let world = app.world_mut();

        let index = world.query::<&TextIndex>().single(world).unwrap();

        assert_eq!(index.0, 0);
    }

    #[test]
    pub fn test_text_only_child_has_index_1() {
        let mut app = App::new();

        app.add_systems(
            Update,
            (
                update_text_roots_system::<Text, TestRoot, TestLayout>,
                update_text_indices_system::<TestRoot>,
            )
                .chain(),
        );

        let world = app.world_mut();

        world.spawn((Text, children![Text]));

        app.update();

        let world = app.world_mut();

        let index = world
            .query_filtered::<&TextIndex, Without<TestRoot>>()
            .single(world)
            .unwrap();

        assert_eq!(index.0, 1);
    }

    #[test]
    pub fn test_text_many_children_indices() {
        let mut app = App::new();

        app.add_systems(
            Update,
            (
                update_text_roots_system::<Text, TestRoot, TestLayout>,
                update_text_indices_system::<TestRoot>,
            )
                .chain(),
        );

        let world = app.world_mut();

        world.spawn((
            Text,
            children![Text, (Text, children![Text, Text]), Text, Text],
        ));

        app.update();

        let world = app.world_mut();

        let parent = world
            .query_filtered::<Entity, With<TestRoot>>()
            .single(world)
            .unwrap();

        let text_children: Vec<Entity> =
            DescendantIter::new(&world.query::<&Children>().query(world), parent)
                .into_iter()
                .collect();

        for (i, child) in text_children.into_iter().enumerate() {
            let index = world.entity(child).get::<TextIndex>().unwrap().0;
            assert_eq!(index, i + 1);
        }
    }
}
