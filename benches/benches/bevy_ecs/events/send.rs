use bevy_ecs::prelude::*;

pub struct Benchmark<const SIZE: usize> {
    events: Events<[u8; SIZE]>,
    count: usize,
}

impl<const SIZE: usize> Benchmark<SIZE> {
    pub fn new(count: usize) -> Self {
        let mut events = Events::default();
        
        // Force both internal buffers to be allocated.
        for _ in 0..2 {
            for _ in 0..count {
                events.send([0u8; SIZE]);
            }
            events.update();
        }

        Self { events, count }
    }

    pub fn run(&mut self) {
        for _ in 0..self.count {
            self.events.send(std::hint::black_box([0u8; SIZE]));
        }
        self.events.update();
    }
}
