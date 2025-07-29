use bevy_ecs::prelude::*;

mod case1 {
    use super::*;

    #[derive(Component, Debug)]
    #[component(on_insert = foo_hook)]
    //~^ ERROR: Custom on_insert hooks are not supported as relationships already define an on_insert hook
    #[relationship(relationship_target = FooTargets)]
    pub struct FooTargetOfFail(Entity);

    #[derive(Component, Debug)]
    #[relationship_target(relationship = FooTargetOfFail)]
    //~^ E0277
    pub struct FooTargets(Vec<Entity>);
}

mod case2 {
    use super::*;

    #[derive(Component, Debug)]
    #[component(on_replace = foo_hook)]
    //~^ ERROR: Custom on_replace hooks are not supported as RelationshipTarget already defines an on_replace hook
    #[relationship_target(relationship = FooTargetOf)]
    pub struct FooTargetsFail(Vec<Entity>);

    #[derive(Component, Debug)]
    #[relationship(relationship_target = FooTargetsFail)]
    //~^ E0277
    pub struct FooTargetOf(Entity);
}

mod case3 {
    use super::*;

    #[derive(Component, Debug)]
    #[component(on_replace = foo_hook)]
    //~^ ERROR: Custom on_replace hooks are not supported as Relationships already define an on_replace hook
    #[relationship(relationship_target = BarTargets)]
    pub struct BarTargetOfFail(Entity);

    #[derive(Component, Debug)]
    #[relationship_target(relationship = BarTargetOfFail)]
    //~^ E0277
    pub struct BarTargets(Vec<Entity>);
}

mod case4 {
    use super::*;

    #[derive(Component, Debug)]
    #[component(on_despawn = foo_hook)]
    //~^ ERROR: Custom on_despawn hooks are not supported as this RelationshipTarget already defines an on_despawn hook, via the 'linked_spawn' attribute
    #[relationship_target(relationship = BarTargetOf, linked_spawn)]
    pub struct BarTargetsFail(Vec<Entity>);

    #[derive(Component, Debug)]
    #[relationship(relationship_target = BarTargetsFail)]
    //~^ E0277
    pub struct BarTargetOf(Entity);
}

fn foo_hook(_world: bevy_ecs::world::DeferredWorld, _ctx: bevy_ecs::lifecycle::HookContext) {}
