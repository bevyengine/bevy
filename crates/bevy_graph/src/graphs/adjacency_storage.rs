/// Adjacency storage enum helper: `Directed` or `Undirected`
#[derive(Clone)]
pub enum AdjacencyStorage<S> {
    /// Undirected graphs share one storage (`S`) for incoming and outgoing edges
    Undirected(S),
    /// Directed graphs have two storages (`S`) for incoming and outgoing edges
    Directed(S, S),
}

impl<S> AdjacencyStorage<S> {
    /// Returns an immutable reference to the incoming adjacency storage
    #[inline]
    pub const fn incoming(&self) -> &S {
        match self {
            AdjacencyStorage::Undirected(storage) => storage,
            AdjacencyStorage::Directed(incoming, _) => incoming,
        }
    }

    /// Returns a mutable reference to the incoming adjacency storage
    #[inline]
    pub fn incoming_mut(&mut self) -> &mut S {
        match self {
            AdjacencyStorage::Undirected(storage) => storage,
            AdjacencyStorage::Directed(incoming, _) => incoming,
        }
    }

    /// Returns an immutable reference to the outgoing adjacency storage
    #[inline]
    pub fn outgoing(&self) -> &S {
        match self {
            AdjacencyStorage::Undirected(storage) => storage,
            AdjacencyStorage::Directed(_, outgoing) => outgoing,
        }
    }

    /// Returns a mutable reference to the outgoing adjacency storage
    #[inline]
    pub fn outgoing_mut(&mut self) -> &mut S {
        match self {
            AdjacencyStorage::Undirected(storage) => storage,
            AdjacencyStorage::Directed(_, outgoing) => outgoing,
        }
    }

    /// Executes a function for each storage
    #[inline]
    pub fn for_each_mut(&mut self, f: fn(&mut S)) {
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
