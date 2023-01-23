use crate::graphs::Graph;

/// Implementation of the [`BFS` algorythm](https://www.geeksforgeeks.org/breadth-first-search-or-bfs-for-a-graph/)
pub mod bfs;
/// Implementation of the [`DFS` algorythm](https://www.geeksforgeeks.org/depth-first-search-or-dfs-for-a-graph/)
pub mod dfs;

/// A special iterator-like trait for iterations over data in graphs.
pub trait GraphIterator<'g, N, E, G: Graph<N, E>> {
    /// The type of the elements being iterated over immutable.
    type Item;

    /// Gets an immutable reference to the value of the next node from the algorithm and advances.
    fn next(&mut self, graph: &'g G) -> Option<Self::Item>;

    /// The type of the elements being iterated over mutable.
    type ItemMut;

    /// Gets a mutable reference to the value of the next node from the algorithm and advances.
    fn next_mut(&mut self, graph: &'g mut G) -> Option<Self::ItemMut>;
}
