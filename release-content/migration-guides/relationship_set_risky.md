---
title: Relationship method set_risky
pull_requests: [19601]
---

The trait `Relationship` received a new method, `set_risky`. It is used to alter the entity ID of the parent of this relationship.
This is needed to keep [other data you can store in these components](https://docs.rs/bevy/latest/bevy/ecs/relationship/trait.Relationship.html#derive)
untouched at operations that reassign the parent, for example `EntityCommands::add_related`. Previously this could have caused the
data to be reset to its default value which may not be what you wanted to happen.

Manually overwriting the component is still possible everywhere the full component is inserted:

```rs
#[derive(Component)]
#[relationship(relationship_target = Parent)]
struct Child {
    #[relationship]
    parent: Entity,
    data: u8,
}

#[derive(Component)]
#[relationship_target(relationship = Child)]
struct Parent(Vec<Entity>);

let mut entity_mut = world.entity_mut(my_entity);

// if `child_entity` already contains `Child`, then its data is unchanged
entity_mut.add_related::<Child>(&[child_entity]);

// if `my_entity` already contains `Child`, then its data is overwritten with 42
entity_mut.insert(Child {
    parent: parent_entity,
    data: 42
});
```

The new method should not be called by user code as that can invalidate the relationship it had or will have.

If you implement `Relationship` manually (which is strongly discouraged) then this method needs to overwrite the `Entity`
used for the relationship.
