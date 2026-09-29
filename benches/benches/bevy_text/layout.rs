use criterion::criterion_group;
use criterion::Criterion;

criterion_group!(benches, layout);

fn layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_layout");
}
