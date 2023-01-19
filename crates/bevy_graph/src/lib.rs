#![warn(missing_docs)]
//! Graph data structures, as used by the Bevy game engine

/// All implemented algorithms for graphs
pub mod algos;
/// All errors that can occur when executing a graph operation
pub mod error;
/// The `Graph` trait and all graph implementations
pub mod graphs;
/// Helping `Iterator`s for graphs
pub mod iters;
/// Utils used by graphs
pub mod utils;
