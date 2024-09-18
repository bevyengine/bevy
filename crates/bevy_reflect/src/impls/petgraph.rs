use crate::{
    impl_reflect_value,
    prelude::ReflectDefault,
    ReflectDeserialize, ReflectSerialize, {self as bevy_reflect},
};

impl_reflect_value!(::petgraph::graph::NodeIndex(
    Default,
    Serialize,
    Deserialize
));
impl_reflect_value!(::petgraph::graph::DiGraph<
    N: ::core::clone::Clone,
    E: ::core::clone::Clone,
    Ix: ::petgraph::graph::IndexType
>());
