use criterion::*;

mod send;
mod iter;

criterion_group!(
    event_benches,
    send,
    iter,
);

fn send(c: &mut Criterion) {
    let mut group = c.benchmark_group("events_send");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for count in [100, 1000, 10000, 50000] {
        group.bench_function(format!("{}", count), |b| {
            let mut bench = send::Benchmark::new(count);
            b.iter(move || bench.run());
        });
    }
    group.finish();
}

fn iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("events_iter");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for count in [100, 1000, 10000, 50000] {
        group.bench_function(format!("{}", count), |b| {
            let mut bench = iter::Benchmark::new(count);
            b.iter(move || bench.run());
        });
    }
    group.finish();
}
