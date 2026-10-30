use criterion::criterion_main;

mod propagate;

criterion_main!(propagate::benches);
