pub mod render;
pub mod asset;
mod app;
mod vertex;
mod core;

pub use app::{App, AppStage, AppBuilder};
pub use crate::core::*;

pub use wgpu;
pub use legion;
pub use legion_transform;
pub use legion::prelude::*;
pub use legion::schedule::Schedulable;
pub use legion_transform::prelude::*;
pub use legion_transform::transform_system_bundle;
pub use glam as math;