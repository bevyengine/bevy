use bevy_graph::{
    algos::{bfs::BreadthFirstSearch, dfs::DepthFirstSearch},
    graphs::{keys::NodeIdx, simple::SimpleMapGraph, Graph},
};
use bevy_utils::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::seq::SliceRandom;

criterion_main!(benches);

const WARM_UP_TIME: Duration = Duration::from_millis(500);

criterion_group! {
    name = benches;
    config = Criterion::default().warm_up_time(WARM_UP_TIME);
    targets = algo_10_000
}

// TODO: find a better way for fix with multiple iterations over same graph
fn algo_10_000(c: &mut Criterion) {
    let mut graph = SimpleMapGraph::<i32, (), false>::new();
    let mut nodes = Vec::with_capacity(10_000);
    let first = graph.new_node(0);
    for i in 1..10_000 {
        nodes.push(graph.new_node(i));
    }
    let mut shuffled = nodes.clone();
    shuffled.shuffle(&mut rand::thread_rng());
    for (i, node) in shuffled
        .iter()
        .cloned()
        .collect::<Vec<NodeIdx>>()
        .windows(2)
        .enumerate()
    {
        let _ = graph.new_edge(node[0], node[1], ()).unwrap();
        let _ = black_box(graph.new_edge(node[0], nodes[i], ()));
    }

    c.bench_function("bfs_10_000", |b| {
        b.iter(|| {
            let mut bfs = BreadthFirstSearch::with_capacity(first, graph.count());
            while let Some(node) = bfs.next(&graph) {
                let _ = black_box(node);
            }
        })
    });

    c.bench_function("dfs_10_000", |b| {
        b.iter(|| {
            let mut dfs = DepthFirstSearch::with_capacity(first, graph.count());
            while let Some(node) = dfs.next(&graph) {
                let _ = black_box(node);
            }
        })
    });
}
