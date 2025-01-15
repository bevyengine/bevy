// TODO: REMOVE THIS
#![allow(missing_docs)]

//! Parent-Child relationships for entities.

use crate as bevy_ecs;
use crate::bundle::Bundle;
use crate::component::ComponentId;
use crate::relationship::{RelatedSpawner, RelatedSpawnerCommands};
use crate::system::EntityCommands;
use crate::world::{DeferredWorld, EntityWorldMut};
use crate::{
    component::Component,
    entity::{Entity, VisitEntities},
    reflect::{
        ReflectComponent, ReflectFromWorld, ReflectMapEntities, ReflectVisitEntities,
        ReflectVisitEntitiesMut,
    },
    relationship::{Relationship, RelationshipSources},
    world::{FromWorld, World},
};
use alloc::{format, string::String, vec::Vec};
use bevy_ecs_macros::VisitEntitiesMut;
use bevy_reflect::Reflect;
use core::slice;
use disqualified::ShortName;
use log::warn;

#[derive(Relationship, Clone, Reflect, VisitEntities, VisitEntitiesMut, PartialEq, Eq, Debug)]
#[reflect(
    Component,
    MapEntities,
    VisitEntities,
    VisitEntitiesMut,
    PartialEq,
    Debug,
    FromWorld
)]
#[relationship_sources(Children)]
pub struct Parent(pub Entity);

impl Parent {
    pub fn get(&self) -> Entity {
        self.0
    }
}

// TODO: We need to impl either FromWorld or Default so Parent can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Parent should only ever be set with a real user-defined entity.  Its worth looking into
// better ways to handle cases like this.
impl FromWorld for Parent {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        Parent(Entity::PLACEHOLDER)
    }
}

#[derive(RelationshipSources, Default, Reflect, VisitEntitiesMut)]
#[relationship(Parent)]
#[despawn_descendants]
#[reflect(Component, MapEntities, VisitEntities, VisitEntitiesMut)]
pub struct Children(Vec<Entity>);

impl<'a> IntoIterator for &'a Children {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl core::ops::Deref for Children {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type ChildSpawner<'w> = RelatedSpawner<'w, Parent>;
pub type ChildSpawnerCommands<'w> = RelatedSpawnerCommands<'w, Parent>;

impl<'w> EntityWorldMut<'w> {
    /// Spawns children of this entity (with a [`Parent`] relationship) by taking a function that operates on a [`ChildSpawner`].
    pub fn with_children(&mut self, func: impl FnOnce(&mut ChildSpawner)) -> &mut Self {
        self.with_related(func);
        self
    }

    /// Adds the given children to this entity
    pub fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        self.add_related::<Parent>(children)
    }

    /// Adds the given child to this entity
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        self.add_related::<Parent>(&[child])
    }

    /// Spawns the passed bundle and adds it to this entity as a child.
    ///
    /// For efficient spawning of multiple children, use [`with_children`].
    ///
    /// [`with_children`]: EntityWorldMut::with_children
    pub fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        let id = self.id();
        self.world_scope(|world| {
            world.spawn((bundle, Parent(id)));
        });
        self
    }

    #[deprecated(since = "0.16.0", note = "Use entity_mut.remove::<Parent>()")]
    pub fn remove_parent(&mut self) -> &mut Self {
        self.remove::<Parent>();
        self
    }

    #[deprecated(since = "0.16.0", note = "Use entity_mut.insert(Parent(entity))")]
    pub fn set_parent(&mut self, parent: Entity) -> &mut Self {
        self.insert(Parent(parent));
        self
    }
}

impl<'a> EntityCommands<'a> {
    /// Spawns children of this entity (with a [`Parent`] relationship) by taking a function that operates on a [`ChildSpawner`].
    pub fn with_children(
        &mut self,
        func: impl FnOnce(&mut RelatedSpawnerCommands<Parent>),
    ) -> &mut Self {
        self.with_related(func);
        self
    }

    /// Adds the given children to this entity
    pub fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        self.add_related::<Parent>(children)
    }

    /// Adds the given child to this entity
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        self.add_related::<Parent>(&[child])
    }

    /// Spawns the passed bundle and adds it to this entity as a child.
    ///
    /// For efficient spawning of multiple children, use [`with_children`].
    ///
    /// [`with_children`]: EntityCommands::with_children
    pub fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        let id = self.id();
        self.commands.spawn((bundle, Parent(id)));
        self
    }

    #[deprecated(since = "0.16.0", note = "Use entity_commands.remove::<Parent>()")]
    pub fn remove_parent(&mut self) -> &mut Self {
        self.remove::<Parent>();
        self
    }

    #[deprecated(since = "0.16.0", note = "Use entity_commands.insert(Parent(entity))")]
    pub fn set_parent(&mut self, parent: Entity) -> &mut Self {
        self.insert(Parent(parent));
        self
    }
}

pub fn validate_parent_has_component<C: Component>(
    world: DeferredWorld,
    entity: Entity,
    _: ComponentId,
) {
    let entity_ref = world.entity(entity);
    let Some(child_of) = entity_ref.get::<Parent>() else {
        return;
    };
    if !world
        .get_entity(child_of.get())
        .map_or(false, |e| e.contains::<C>())
    {
        // TODO: print name here once Name lives in bevy_ecs
        let name: Option<String> = None;
        warn!(
            "warning[B0004]: {name} with the {ty_name} component has a parent without {ty_name}.\n\
            This will cause inconsistent behaviors! See: https://bevyengine.org/learn/errors/b0004",
            ty_name = ShortName::of::<C>(),
            name = name.map_or_else(
                || format!("Entity {}", entity),
                |s| format!("The {s} entity")
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::Entity,
        hierarchy::{Children, Parent},
        relationship::RelationshipSources,
        world::World,
    };
    use alloc::{vec, vec::Vec};

    #[derive(PartialEq, Eq, Debug)]
    struct Node {
        entity: Entity,
        children: Vec<Node>,
    }

    impl Node {
        fn new(entity: Entity) -> Self {
            Self {
                entity,
                children: Vec::new(),
            }
        }

        fn new_with(entity: Entity, children: Vec<Node>) -> Self {
            Self { entity, children }
        }
    }

    fn get_hierarchy(world: &World, entity: Entity) -> Node {
        Node {
            entity,
            children: world
                .entity(entity)
                .get::<Children>()
                .map_or_else(Default::default, |c| {
                    c.iter().map(|e| get_hierarchy(world, e)).collect()
                }),
        }
    }

    #[test]
    fn hierarchy() {
        let mut world = World::new();
        let id = world.spawn_empty().id();
        let id1 = world.spawn(Parent(id)).id();
        let id3 = world.spawn(Parent(id1)).id();
        let id2 = world.spawn(Parent(id)).id();

        // Spawn
        let hierarchy = get_hierarchy(&world, id);
        assert_eq!(
            hierarchy,
            Node::new_with(
                id,
                vec![Node::new_with(id1, vec![Node::new(id3)]), Node::new(id2)]
            )
        );

        // Removal
        world.entity_mut(id1).remove::<Parent>();
        let hierarchy = get_hierarchy(&world, id);
        assert_eq!(hierarchy, Node::new_with(id, vec![Node::new(id2)]));

        // Insert
        world.entity_mut(id1).insert(Parent(id));
        let hierarchy = get_hierarchy(&world, id);
        assert_eq!(
            hierarchy,
            Node::new_with(
                id,
                vec![Node::new(id2), Node::new_with(id1, vec![Node::new(id3)])]
            )
        );

        // Recursive Despawn
        world.entity_mut(id).despawn();
        assert!(world.get_entity(id).is_err());
        assert!(world.get_entity(id1).is_err());
        assert!(world.get_entity(id2).is_err());
        assert!(world.get_entity(id3).is_err());
    }

    #[test]
    fn self_parenting_invalid() {
        let mut world = World::new();
        let id = world.spawn_empty().id();
        world.entity_mut(id).insert(Parent(id));
        assert!(
            world.entity(id).get::<Parent>().is_none(),
            "invalid Parent relationships should self-remove"
        );
    }

    #[test]
    fn missing_parent_invalid() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        world.entity_mut(parent).despawn();
        let id = world.spawn(Parent(parent)).id();
        assert!(
            world.entity(id).get::<Parent>().is_none(),
            "invalid Parent relationships should self-remove"
        );
    }
}
