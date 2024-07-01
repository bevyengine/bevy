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
    fn get_components(&mut self, world: &mut World) {
        let q1 = self.state_r.get(&world);
        let a1 = q1.iter().next().unwrap();

        let mut q2 = self.state_w.get_mut(world);
        //~^ E0502
        let _ = q2.iter_mut().next().unwrap();

        println!("{}", a1.0);
    }
}
