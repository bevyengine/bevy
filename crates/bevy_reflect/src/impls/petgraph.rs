use crate::{impl_reflect_opaque, prelude::ReflectDefault, ReflectDeserialize, ReflectSerialize};

impl_reflect_opaque!(::petgraph::graph::NodeIndex(
    Clone,
    Default,
    PartialEq,
    Hash,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::petgraph::graph::DiGraph<
    N: ::core::clone::Clone,
    E: ::core::clone::Clone,
    Ix: ::petgraph::graph::IndexType
>(Clone));
