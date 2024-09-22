use criterion::*;

mod iter;
mod send;

criterion_group!(event_benches, send, iter);

fn send(c: &mut Criterion) {
    let mut group = c.benchmark_group("events_send");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for count in [100, 1000, 10000, 50000] {
        group.bench_function(format!("size_4_events_{}", count), |b| {
            let mut bench = send::Benchmark::<4>::new(count);
            b.iter(move || bench.run());
        });
    }
    for count in [100, 1000, 10000, 50000] {
        group.bench_function(format!("size_16_events_{}", count), |b| {
            let mut bench = send::Benchmark::<16>::new(count);
            b.iter(move || bench.run());
        });
    }
    for count in [100, 1000, 10000, 50000] {
        group.bench_function(format!("size_512_events_{}", count), |b| {
            let mut bench = send::Benchmark::<512>::new(count);
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
        group.bench_function(format!("size_4_events_{}", count), |b| {
            let mut bench = iter::Benchmark::<4>::new(count);
            b.iter(move || bench.run());
        });
    }
    for count in [100, 1000, 10000, 50000] {
        group.bench_function(format!("size_16_events_{}", count), |b| {
            let mut bench = iter::Benchmark::<4>::new(count);
            b.iter(move || bench.run());
        });
    }
    for count in [100, 1000, 10000, 50000] {
        group.bench_function(format!("size_512_events_{}", count), |b| {
            let mut bench = iter::Benchmark::<512>::new(count);
            b.iter(move || bench.run());
        });
    }
    group.finish();
}
