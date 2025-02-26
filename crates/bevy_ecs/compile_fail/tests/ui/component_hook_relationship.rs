use bevy_ecs::prelude::*;

#[derive(Component, Debug)]
#[relationship(relationship_target = FooTargets)]
#[component(
    on_insert = |w,ctx| {},
    //~^ E0001
    on_replace = |w,ctx| {},
    //~^ E0001
)]
pub struct FooTargetOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = FooTargetOf)]
#[component(
    on_replace = |w,ctx| {},
    //~^ E0001
    on_despawn = |w,ctx| {},
    //~^ E0001
)]
pub struct FooTargets(Vec<Entity>);
