use bevy_ecs::prelude::*;
use bevy_ecs::system::{ReadOnlySystemParam, SystemParam, SystemState};

#[derive(Component)]
struct Foo;

#[derive(SystemParam)]
struct Mutable<'w, 's> {
    a: Query<'w, 's, &'static mut Foo>,
}

fn main() {

    let mut world = World::default();
    let state = SystemState::<Mutable>::new(&mut world);
    state.get(&world);
    //~^ E0277
}

fn assert_readonly<P>()
where
    P: ReadOnlySystemParam,
{
}
