use criterion::criterion_main;

mod bezier;
mod easing;

criterion_main!(bezier::benches, easing::benches);
