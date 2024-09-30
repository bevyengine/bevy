use crate::{
    self as bevy_reflect, impl_reflect_opaque, prelude::ReflectDefault, ReflectDeserialize,
    ReflectSerialize,
};

impl_reflect_opaque!(::petgraph::graph::NodeIndex(
    Default,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::petgraph::graph::DiGraph<
    N: ::core::clone::Clone,
    E: ::core::clone::Clone,
    Ix: ::petgraph::graph::IndexType
>());
