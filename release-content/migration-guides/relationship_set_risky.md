---
title: Relationship method set_risky
pull_requests: [19601]
---

The trait `Relationship` received a new method, `set_risky`. It is used to alter the entity ID of the entity that contains its `RelationshipTarget` counterpart.
This is needed to leave [other data you can store in these components](https://docs.rs/bevy/latest/bevy/ecs/relationship/trait.Relationship.html#derive)
unchanged at operations that reassign the relationship target, for example `EntityCommands::add_related`. Previously this could have caused the
data to be reset to its default value which may not be what you wanted to happen.

Manually overwriting the component is still possible everywhere the full component is inserted:

```rs
#[derive(Component)]
#[relationship(relationship_target = CarOwner)]
struct OwnedCar {
    #[relationship]
    owner: Entity,
    first_owner: Option<Entity>, // None if `owner` is the first one
}

#[derive(Component)]
#[relationship_target(relationship = OwnedCar)]
struct CarOwner(Vec<Entity>);

let mut me_entity_mut = world.entity_mut(me_entity);

// if `car_entity` already contains `OwnedCar`, then the first owner remains unchanged
me_entity_mut.add_one_related::<OwnedCar>(car_entity);

// if `car_entity` already contains `OwnedCar`, then the first owner is overwritten with None here
car_entity_mut.insert(OwnedCar {
    owner: me_entity,
    first_owner: None // I swear it is not stolen officer!
});
```

The new method should not be called by user code as that can invalidate the relationship it had or will have.

If you implement `Relationship` manually (which is strongly discouraged) then this method needs to overwrite the `Entity`
used for the relationship.
