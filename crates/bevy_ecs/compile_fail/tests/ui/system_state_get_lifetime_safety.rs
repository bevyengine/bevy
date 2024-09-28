use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;

#[derive(Component)]
struct A(usize);

#[derive(Component)]
struct B(usize);

struct State {
    state_r: SystemState<Query<'static, 'static, &'static A>>,
    state_w: SystemState<Query<'static, 'static, &'static mut A>>,
}

impl State {
    fn get_component(&mut self, world: &mut World, entity: Entity) {
        let q1 = self.state_r.get(&world);
        let a1 = q1.get(entity).unwrap();

        let mut q2 = self.state_w.get_mut(world);
        //~^ E0502
        let _ = q2.get_mut(entity).unwrap();

        println!("{}", a1.0);
    }
}
