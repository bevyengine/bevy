use bevy_ecs::prelude::*;

#[derive(Event)]
struct BenchEvent<const SIZE: usize>([u8; SIZE]);

pub struct ReaderBenchmark<const SIZE: usize>(Events<BenchEvent<SIZE>>);

impl<const SIZE: usize> ReaderBenchmark<SIZE> {
    pub fn new(count: usize) -> Self {
        let mut events = Events::default();

        for _ in 0..count {
            events.send(BenchEvent([0u8; SIZE]));
        }

        Self(events)
    }

    pub fn run(&mut self) {
        let mut reader = self.0.get_reader();
        let last = reader.last(&self.0);
        std::hint::black_box(last);
    }
}

pub struct IterBenchmark<const SIZE: usize>(Events<BenchEvent<SIZE>>);

impl<const SIZE: usize> IterBenchmark<SIZE> {
    pub fn new(count: usize) -> Self {
        let mut events = Events::default();

        for _ in 0..count {
            events.send(BenchEvent([0u8; SIZE]));
        }

        Self(events)
    }

    pub fn run(&mut self) {
        let mut reader = self.0.get_reader();
        let last = reader.read(&self.0).last();
        std::hint::black_box(last);
    }
}
