/// Adjacency storage enum helper: `Directed` or `Undirected`
#[derive(Clone)]
pub enum AdjacencyStorage<S> {
    /// Undirected graphs share one storage (`S`) for incoming and outgoing edges
    Undirected(S),
    /// Directed graphs have two storages (`S`) for incoming and outgoing edges
    Directed(S, S),
}

impl<S> AdjacencyStorage<S> {
    #[inline]
    pub const fn incoming_mut(&mut self) -> &mut S {
        match self {
            AdjacencyStorage::Undirected(storage) => storage,
            AdjacencyStorage::Directed(incoming, _) => incoming,
        }
    }

    #[inline]
    pub const fn outgoing_mut(&mut self) -> &mut S {
        match self {
            AdjacencyStorage::Undirected(storage) => storage,
            AdjacencyStorage::Directed(_, outgoing) => outgoing,
        }
    }

    #[inline]
    pub const fn for_each_mut(&mut self, f: fn(&mut S)) {
        match self {
            AdjacencyStorage::Undirected(storage) => {
                f(storage);
            }
            AdjacencyStorage::Directed(incoming, outgoing) => {
                f(incoming);
                f(outgoing);
            }
        }
    }
}
