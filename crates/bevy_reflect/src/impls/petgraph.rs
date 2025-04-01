use crate::{ReflectDeserialize, ReflectSerialize, impl_reflect_opaque, prelude::ReflectDefault};

impl_reflect_opaque!(::petgraph::graph::NodeIndex(
    Clone,
    Default,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::petgraph::graph::DiGraph<
    N: ::core::clone::Clone,
    E: ::core::clone::Clone,
    Ix: ::petgraph::graph::IndexType
>(Clone));
