use crate::{self as bevy_reflect, prelude::ReflectDefault};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::petgraph::graph::NodeIndex(Default));
