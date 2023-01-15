#[macro_export]
macro_rules! impl_graph {
    (impl COMMON for $name:ident {
        $($common_code:tt)*
    }

    $(impl COMMON?undirected {
        $($common_undirected_code:tt)*
    })?

    $(impl COMMON?directed {
        $($common_directed_code:tt)*
    })?

    $(
        impl SIMPLE {
            $($simple_code:tt)*
        }

        $(impl SIMPLE?undirected {
            $($simple_undirected_code:tt)*
        })?

        $(impl SIMPLE?directed {
            $($simple_directed_code:tt)*
        })?
    )?) => {
        impl<N, E> $crate::graphs::Graph<N, E> for $name<N, E, false> {
            $($common_code)*

            $($($common_undirected_code)*)?
        }

        impl<N, E> $crate::graphs::Graph<N, E> for $name<N, E, true> {
            $($common_code)*

            $($($common_directed_code)*)?
        }

        $(
            impl<N, E> $crate::graphs::SimpleGraph<N, E> for $name<N, E, false> {
                $($simple_code)*

                $($($simple_undirected_code)*)?
            }

            impl<N, E> $crate::graphs::SimpleGraph<N, E> for $name<N, E, true> {
                $($simple_code)*

                $($($simple_directed_code)*)?
            }
        )?
    }
}
