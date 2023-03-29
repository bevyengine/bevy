use bevy_ecs::prelude::*;

struct A(f32);

pub struct Benchmark(Events<A>, usize);

impl Benchmark {
    pub fn new(count: usize) -> Self {
        let mut events = Events::default();
        
        // Force the internal buffers to be allocated
        for _ in 0..2 {
            for _ in 0..count {
                events.send(A(0.0));
            }
            events.update();
        }

        Self(events, count)
    }

    pub fn run(&mut self) {
        for _ in 0..self.1 {
            self.0.send(A(0.0));
        }
        self.0.update();
    }
}
