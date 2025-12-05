use bevy_ecs::prelude::*;

// the proc macro allows general paths, which means normal structs are also passing the basic
// parsing. This test makes sure that we don't accidentally allow structs as hooks through future
// changes.
//
// Currently the error is thrown in the generated code and not while executing the proc macro
// logic.
#[derive(Component)]
#[component(
    on_add = Bar,
    //~^ E0425
)]
pub struct FooWrongPath;
