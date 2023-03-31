use bevy_ecs::prelude::*;

pub struct Benchmark<const SIZE: usize>(Events<[u8; SIZE]>);

impl<const SIZE: usize> Benchmark<SIZE> {
    pub fn new(count: usize) -> Self {
        let mut events = Events::default();

        for _ in 0..count {
            events.send([0u8; SIZE]);
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
