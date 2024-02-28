use crate::{
    self as bevy_reflect, impl_reflect_value, prelude::ReflectDefault, ReflectDeserialize,
    ReflectSerialize,
};

impl_reflect_value!(::petgraph::graph::NodeIndex(
    Default,
    Serialize,
    Deserialize
));
impl_reflect_value!(::petgraph::graph::DiGraph<
    N: ::std::clone::Clone,
    E: ::std::clone::Clone,
    Ix: ::petgraph::graph::IndexType
>());
