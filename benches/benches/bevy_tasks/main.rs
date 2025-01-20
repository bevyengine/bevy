use criterion::criterion_main;

mod iter;

criterion_main!(iter::benches);
