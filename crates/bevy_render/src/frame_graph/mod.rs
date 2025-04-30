pub mod blue_pritnt;
pub mod common;
pub mod device_pass;
pub mod graph;
pub mod handle;
pub mod pass;
pub mod pass_node;
pub mod pass_node_builder;
pub mod render_context;
pub mod resource;
pub mod resource_board;
pub mod resource_node;
pub mod resource_table;
pub mod transient_resource_cache;

pub use blue_pritnt::*;
pub use common::*;
pub use device_pass::*;
pub use graph::*;
pub use handle::*;
pub use pass::*;
pub use pass_node::*;
pub use pass_node_builder::*;
pub use render_context::*;
pub use resource::*;
pub use resource_board::*;
pub use resource_node::*;
pub use resource_table::*;
pub use transient_resource_cache::*;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum FrameGraphError {
    #[error("ResourceNotFound")]
    ResourceNotFound,
    #[error("{key:?} not put int FrameGraph")]
    ResourceBoardKey { key: ResourceBoardKey },
}
