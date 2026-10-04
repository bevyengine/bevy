use criterion::criterion_main;

mod immediate_mode;

criterion_main!(immediate_mode::benches);
