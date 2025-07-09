use bevy_ecs::prelude::*;

#[derive(Event, BufferedEvent)]
struct BenchEvent<const SIZE: usize>([u8; SIZE]);

pub struct Benchmark<const SIZE: usize>(Events<BenchEvent<SIZE>>);

impl<const SIZE: usize> Benchmark<SIZE> {
    pub fn new(count: usize) -> Self {
        let mut events = Events::default();

        for _ in 0..count {
            events.write(BenchEvent([0u8; SIZE]));
        }

        Self(events)
    }

    pub fn run(&mut self) {
        let mut reader = self.0.get_cursor();
        for evt in reader.read(&self.0) {
            core::hint::black_box(evt);
        }
    }
}
