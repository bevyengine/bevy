use criterion::criterion_main;

mod propagation;

criterion_main!(propagation::benches);
