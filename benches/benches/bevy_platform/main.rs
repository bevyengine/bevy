use criterion::criterion_main;

mod aligned_vec;

criterion_main!(aligned_vec::benches);
