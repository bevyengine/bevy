use crate::{text, Font, TextFont, TextLayoutInfo};
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, reflect::ReflectComponent, relationship::Relationship};
use bevy_reflect::prelude::*;
use bevy_utils::{default, once};
use cosmic_text::{Buffer, Metrics};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tracing::warn;

/// Index of a text entity in a text layout
#[derive(Component, Debug, PartialEq)]
pub struct TextIndex(usize);
#[derive(Component, Debug, PartialEq)]
pub struct TextSections(SmallVec<[Entity; 1]>);

/// Update text roots
pub fn identify_text_roots_system<T: Component, Root: RelationshipTarget, Layout: Relationship>(
    mut commands: Commands,
    orphan_query: Query<Entity, (With<T>, Without<ChildOf>, Without<Root>)>,
    child_query: Query<(Entity, &ChildOf, Has<Root>), With<T>>,
    parent_query: Query<&T>,
    non_text_root_query: Query<Entity, (With<Root>, Without<T>)>,
) {
    for text_orphan in orphan_query.iter() {
        commands.spawn(Layout::from(text_orphan));
    }

    for (entity, child_of, has_root) in child_query.iter() {
        let parent_is_text = parent_query.contains(child_of.get());
        if parent_is_text && has_root {
            // entity is not a root
            commands.entity(entity).remove::<Root>();
        } else if !parent_is_text && !has_root {
            // Root entity is not already a root
            commands.spawn(Layout::from(entity));
        }
    }

    for entity in non_text_root_query.iter() {
        commands.entity(entity).remove::<Root>();
    }
}

pub fn update_text_indices<Root: RelationshipTarget>(
    root_query: Query<Entity, With<Root>>,
    descendants: Query<&Children, With<TextIndex>>,
    mut text_index_query: Query<&mut TextIndex>,
) {
    for root_entity in root_query.iter() {
        text_index_query.get_mut(root_entity).ok().unwrap().0 = 0;

        for (index, text_entity) in descendants.iter_descendants(root_entity).enumerate() {
            text_index_query.get_mut(text_entity).ok().unwrap().0 = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextFont;
    use bevy_app::{App, Update};
    use bevy_asset::Handle;
    use bevy_color::Color;
    use bevy_derive::{Deref, DerefMut};
    use bevy_ecs::{prelude::*, reflect::ReflectComponent, relationship::Relationship};
    use bevy_reflect::prelude::*;
    use bevy_utils::{default, once};
    use cosmic_text::{Buffer, Metrics};
    use serde::{Deserialize, Serialize};
    use tracing::warn;
    #[derive(Component, Debug, PartialEq, Eq)]
    #[relationship_target(relationship = TestLayout, linked_spawn)]
    struct TestRoot(Entity);

    #[derive(Component, Debug, PartialEq, Eq)]
    #[relationship(relationship_target = TestRoot)]
    struct TestLayout(Entity);

    #[test]
    pub fn test_identify_text_roots() {
        let mut app = App::new();

        app.add_systems(
            Update,
            identify_text_roots_system::<TextFont, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(TextFont::default()).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world
            .query::<(&TextFont, &TestRoot)>()
            .single(world)
            .unwrap();

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
            identify_text_roots_system::<TextFont, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(TextFont::default()).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world
            .query::<(&TextFont, &TestRoot)>()
            .single(world)
            .unwrap();

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
            identify_text_roots_system::<TextFont, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world
            .spawn((
                TextFont::default(),
                children![TextFont::default(), TextFont::default()],
            ))
            .id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world
            .query::<(&TextFont, &TestRoot)>()
            .single(world)
            .unwrap();

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
            identify_text_roots_system::<TextFont, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let non_text_parent_id = world
            .spawn((children![(
                TextFont::default(),
                children![TextFont::default(), TextFont::default()],
            )],))
            .id();

        app.update();

        let world = app.world_mut();

        let (root_id, _, root) = world
            .query::<(Entity, &TextFont, &TestRoot)>()
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
            identify_text_roots_system::<TextFont, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(TextFont::default()).id();

        app.update();

        let world = app.world_mut();

        let (_, root) = world
            .query::<(&TextFont, &TestRoot)>()
            .single(world)
            .unwrap();

        let target_id = root.0;

        let (layout_id, layout) = world
            .query::<(Entity, &TestLayout)>()
            .single(world)
            .unwrap();

        assert_eq!(target_id, layout_id);
        assert_eq!(root_id, layout.0);

        let parent_id = world.spawn(TextFont::default()).add_child(root_id).id();

        app.update();
        let world = app.world_mut();

        let (new_root_id, _, root) = world
            .query::<(Entity, &TextFont, &TestRoot)>()
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
            identify_text_roots_system::<TextFont, TestRoot, TestLayout>,
        );

        let world = app.world_mut();

        let root_id = world.spawn(TextFont::default()).id();

        app.update();
        let world = app.world_mut();

        assert_eq!(world.query::<&TestRoot>().iter(world).count(), 1);
        assert_eq!(world.query::<&TestLayout>().iter(world).count(), 1);

        world.entity_mut(root_id).remove::<TextFont>();

        app.update();
        let world = app.world_mut();

        assert_eq!(world.query::<&TestRoot>().iter(world).count(), 0);
        assert_eq!(world.query::<&TestLayout>().iter(world).count(), 0);
    }
}
