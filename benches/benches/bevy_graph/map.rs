use bevy_graph::graphs::{
    keys::{EdgeIdx, NodeIdx},
    simple::SimpleMapGraph,
    Graph, UndirectedGraph,
};
use bevy_utils::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

criterion_main!(benches);

const WARM_UP_TIME: Duration = Duration::from_millis(3000);

criterion_group! {
    name = benches;
    config = Criterion::default().warm_up_time(WARM_UP_TIME);
    targets = nodes_10_000_undirected
}

fn nodes_10_000_undirected(c: &mut Criterion) {
    let mut map_graph = SimpleMapGraph::<i32, (), false>::new();

    let mut nodes: Vec<NodeIdx> = Vec::with_capacity(10_000);
    c.bench_function("nodes_10_000_new_node", |b| {
        b.iter(|| {
            for i in 1..=10_000 {
                nodes.push(map_graph.new_node(i));
            }
        })
    });
    let mut edges: Vec<EdgeIdx> = Vec::with_capacity(10_000 - 1);
    c.bench_function("nodes_10_000_new_edge", |b| {
        b.iter(|| {
            for i in 1..10_000 {
                edges.push(map_graph.new_edge(nodes[i - 1], nodes[i], ()));
            }
        })
    });
    c.bench_function("nodes_10_000_get_edge", |b| {
        b.iter(|| {
            for edge in &edges {
                black_box(edge.get(&map_graph));
            }
        })
    });
    c.bench_function("nodes_10_000_remove_edge", |b| {
        b.iter(|| {
            for i in 1..10_000 {
                black_box(
                    map_graph
                        .edge_between(nodes[i - 1], nodes[i])
                        .remove_undirected(&mut map_graph)
                        .unwrap(),
                );
            }
        })
    });
}
