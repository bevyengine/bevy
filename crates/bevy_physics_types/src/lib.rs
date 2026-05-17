#![doc = include_str!("../README.md")]

#[macro_use]
mod macros;

pub mod types;
pub use types::*;

pub mod axis;

pub mod global;

pub mod scene;

pub mod rigid_body;

pub mod mass;

pub mod collision;

pub mod mesh_collision;

pub mod material;

pub mod collision_group;

pub mod filtered_pairs;

pub mod joint;

pub mod joint_revolute;

pub mod joint_prismatic;

pub mod joint_spherical;

pub mod joint_distance;

pub mod joint_fixed;

pub mod limit;

pub mod drive;

pub mod articulation;
