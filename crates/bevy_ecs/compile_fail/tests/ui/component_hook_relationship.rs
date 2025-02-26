use bevy_ecs::prelude::*;

#[derive(Component, Debug)]
#[relationship(relationship_target = FooTargets)]
//~^ ERROR: Custom on_insert hooks are not supported as Relationships already define an on_insert hook
#[component(
    on_insert = |w,ctx| {},
)]
pub struct FooTargetOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = FooTargetOf)]
//~^ ERROR: Custom on_replace hooks are not supported as RelationshipTarget already defines an on_replace hook
#[component(
    on_replace = |w,ctx| {},
)]
pub struct FooTargets(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target = BarTargets)]
//~^ ERROR: Custom on_replace hooks are not supported as Relationships already define an on_replace hook
#[component(
    on_replace = |w,ctx| {},
)]
pub struct BarTargetOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = BarTargetOf)]
//~^ ERROR: Custom on_despawn hooks are not supported as this RelationshipTarget already defines an on_despawn hook, via the 'linked_spawn' attribute
#[component(
    on_despawn = |w,ctx| {},
)]
pub struct BarTargets(Vec<Entity>);
