use criterion::criterion_main;

mod primitives;

criterion_main!(primitives::benches);
