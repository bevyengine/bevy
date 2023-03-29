use bevy_ecs::prelude::*;

struct A(f32);

pub struct Benchmark(Events<A>);

impl Benchmark {
    pub fn new(count: usize) -> Self {
        let mut events = Events::default();
        
        for _ in 0..count {
            events.send(A(0.0));
        }

        Self(events)
    }

    pub fn run(&mut self) {
        let mut reader = self.0.get_reader();
        for evt in reader.iter(&self.0) {
            std::hint::black_box(evt);
        }
    }
}
