#[macro_export]
macro_rules! impl_graph {
    (impl common for $name:ident {
        $($common_code:tt)*
    }

    impl undirected {
        $($undirected_code:tt)*
    }

    impl directed {
        $($directed_code:tt)*
    }) => {
        impl<N, E> crate::graphs::Graph<N, E> for $name<N, E, false> {
            $($common_code)*

            $($undirected_code)*
        }

        impl<N, E> crate::graphs::Graph<N, E> for $name<N, E, true> {
            $($common_code)*

            $($directed_code)*
        }
    }
}

// TODO: replace `crate::graphs::` with one of bevy's macro util helpers
