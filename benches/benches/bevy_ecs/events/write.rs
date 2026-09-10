use bevy_ecs::prelude::*;

#[derive(Message)]
struct BenchEvent<const SIZE: usize>([u8; SIZE]);

impl<const SIZE: usize> Default for BenchEvent<SIZE> {
    fn default() -> Self {
        BenchEvent([0; SIZE])
    }
}

pub struct Benchmark<const SIZE: usize> {
    events: Messages<BenchEvent<SIZE>>,
    count: usize,
}

impl<const SIZE: usize> Benchmark<SIZE> {
    pub fn new(count: usize) -> Self {
        let mut events = Messages::default();

        // Force both internal buffers to be allocated.
        for _ in 0..2 {
            for _ in 0..count {
                events.write(BenchEvent([0u8; SIZE]));
            }
            events.update();
        }

        Self { events, count }
    }

    pub fn run(&mut self) {
        for _ in 0..self.count {
            self.events
                .write(core::hint::black_box(BenchEvent([0u8; SIZE])));
        }
        self.events.update();
    }
}
