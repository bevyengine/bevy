#[macro_export]
macro_rules! impl_graph {
    ($target:ident, $directed:tt) => {
        impl<N, E> Graph<N, E> for $target<N, E, $directed> {
            #[inline(always)]
            fn len(&self) -> usize {
                $target::<N, E, $directed>::len(self)
            }

            #[inline(always)]
            fn new_node(&mut self, node: N) -> NodeIdx {
                $target::<N, E, $directed>::new_node(self, node)
            }

            #[inline(always)]
            fn node(&self, idx: NodeIdx) -> GraphResult<&N> {
                $target::<N, E, $directed>::node(self, idx)
            }

            #[inline(always)]
            fn node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N> {
                $target::<N, E, $directed>::node_mut(self, idx)
            }

            #[inline(always)]
            fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx {
                $target::<N, E, $directed>::new_edge(self, from, to, edge)
            }

            #[inline(always)]
            fn get_edge(&self, edge: EdgeIdx) -> Option<&E> {
                $target::<N, E, $directed>::get_edge(self, edge)
            }

            #[inline(always)]
            fn get_edge_mut(&mut self, edge: EdgeIdx) -> Option<&mut E> {
                $target::<N, E, $directed>::get_edge_mut(self, edge)
            }

            #[inline(always)]
            fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E> {
                $target::<N, E, $directed>::remove_edge(self, edge)
            }

            #[inline(always)]
            fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx {
                $target::<N, E, $directed>::edge_between(self, from, to)
            }

            #[inline(always)]
            fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)> {
                $target::<N, E, $directed>::edges_of(self, node)
            }
        }
    };
}
