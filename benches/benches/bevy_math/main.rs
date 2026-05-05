use criterion::criterion_main;

mod bezier;
mod bounding;

criterion_main!(bezier::benches, bounding::benches);
