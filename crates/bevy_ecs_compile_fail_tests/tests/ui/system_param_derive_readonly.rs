use bevy_ecs::prelude::*;
use bevy_ecs::system::{ReadOnlySystemParam, SystemParam, SystemState};

#[derive(Component)]
struct Foo;

#[derive(SystemParam)]
struct Mutable<'w, 's> {
    a: Query<'w, 's, &'static mut Foo>,
}

fn main() {
    // Ideally we'd use:
    // let mut world = World::default();
    // let state = SystemState::<Mutable>::new(&mut world);
    // state.get(&world);
    // But that makes the test show absolute paths
    assert_readonly::<Mutable>();
}

fn assert_readonly<P>()
where
    P: ReadOnlySystemParam,
{
}
