// FIXME(15321): solve CI failures, then replace with `#![expect()]`.
#![allow(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![allow(unsafe_code)]

extern crate alloc;
extern crate core;

pub mod bounding;
mod conversions;
mod index;
mod mesh;
mod mikktspace;
pub mod morph;
pub mod primitives;
pub mod skinning;
mod vertex;
pub use index::*;
pub use mesh::*;
pub use mikktspace::*;
pub use primitives::*;
pub use vertex::*;
