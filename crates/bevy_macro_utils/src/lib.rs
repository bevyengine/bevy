#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! A collection of helper types and functions for working on macros within the Bevy ecosystem.

extern crate alloc;
extern crate proc_macro;

mod attrs;
mod bevy_manifest;
pub mod fq_std;
mod label;
mod parser;
mod result_sifter;
mod shape;
mod symbol;

pub use attrs::*;
pub use bevy_manifest::*;
pub use label::*;
pub use parser::*;
pub use result_sifter::*;
pub use shape::*;
pub use symbol::*;
