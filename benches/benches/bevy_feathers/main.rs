use criterion::criterion_main;

#[cfg(feature = "experimental_bevy_feathers")]
mod spawn;

#[cfg(feature = "experimental_bevy_feathers")]
criterion_main!(spawn::benches);
