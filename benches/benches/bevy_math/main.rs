use criterion::criterion_main;

mod bezier;

criterion_main!(bezier::benches);
