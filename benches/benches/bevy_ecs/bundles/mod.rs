use criterion::criterion_group;

mod insert_many;
mod spawn_many;
mod spawn_many_zst;
mod spawn_one_zst;

criterion_group!(
    benches,
    spawn_one_zst::spawn_one_zst,
    spawn_many_zst::spawn_many_zst,
    spawn_many::spawn_many,
    insert_many::insert_many,
);
