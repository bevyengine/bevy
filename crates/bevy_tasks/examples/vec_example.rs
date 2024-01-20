use bevy_tasks::{IntoParallelRefMutIterator, ParallelIterator};

fn main() {
    let mut a = vec![2, 3, 4];
    let c = a.par_iter_mut();
    c.for_each(|a| {});
}
