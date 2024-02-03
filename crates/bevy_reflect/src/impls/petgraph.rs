use crate::{self as bevy_reflect};
use bevy_reflect_derive::impl_reflect_value;
use petgraph::stable_graph::IndexType;

impl_reflect_value!(::petgraph::stable_graph::StableGraph<N: Clone, E: Clone> ());

impl_reflect_value!(::petgraph::stable_graph::NodeIndex<I: Clone + IndexType> ());

impl_reflect_value!(::petgraph::stable_graph::EdgeIndex<I: Clone + IndexType> ());
