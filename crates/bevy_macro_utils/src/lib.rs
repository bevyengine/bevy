#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![deny(unsafe_code)]
//! A collection of helper types and functions for working on macros within the Bevy ecosystem.

extern crate proc_macro;

mod attrs;
mod bevy_manifest;
pub mod fq_std;
mod label;
mod shape;
mod symbol;

pub use attrs::*;
pub use bevy_manifest::*;
pub use label::*;
pub use shape::*;
pub use symbol::*;
