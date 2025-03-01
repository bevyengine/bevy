use bevy_ecs::prelude::*;

#[derive(Component, Debug)]
#[relationship(relationship_target = FooTargets)]
//~^ ERROR: Custom on_insert hooks are not supported as Relationships already define an on_insert hook
#[component(on_insert = foo_hook)]
pub struct FooTargetOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = FooTargetOf)]
//~^ ERROR: Custom on_replace hooks are not supported as RelationshipTarget already defines an on_replace hook
#[component(on_replace = foo_hook)]
pub struct FooTargets(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target = BarTargets)]
//~^ ERROR: Custom on_replace hooks are not supported as Relationships already define an on_replace hook
#[component(on_replace = foo_hook)]
pub struct BarTargetOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = BarTargetOf)]
//~^ ERROR: Custom on_despawn hooks are not supported as this RelationshipTarget already defines an on_despawn hook, via the 'linked_spawn' attribute
#[component(on_despawn = foo_hook)]
pub struct BarTargets(Vec<Entity>);

fn foo_hook(world: bevy_ecs::world::DeferredWorld, ctx: bevy_ecs::component::HookContext) {}
