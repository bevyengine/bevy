use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;

#[derive(Component, Eq, PartialEq, Debug, Clone, Copy)]
struct A(usize);

fn main() {
    let mut world = World::default();
    world.spawn(A(1));
    world.spawn(A(2));

    let mut system_state = SystemState::<Query<&mut A>>::new(&mut world);
    {
        let mut query = system_state.get_mut(&mut world);
        let mut_vec = query.iter_mut().collect::<Vec<bevy_ecs::prelude::Mut<A>>>();
        assert_eq!(
            // this should fail to compile due to the later use of mut_vec
            query.iter().collect::<Vec<&A>>(),
            //~^ E0502
            vec![&A(1), &A(2)],
            "both components returned by iter of &mut"
        );
        assert_eq!(
            mut_vec.iter().map(|m| **m).collect::<Vec<A>>(),
            vec![A(1), A(2)],
            "both components returned by iter_mut of &mut"
        );
    }
}
