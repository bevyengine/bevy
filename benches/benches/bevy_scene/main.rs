use criterion::criterion_main;

mod spawn;

criterion_main!(spawn::benches);
