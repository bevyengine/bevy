//! Experimental functionality related to mipmap generation.
//!
//! Currently, this consists only of hierarchical Z buffer generation.

pub mod depth;
mod depth_2mips_fallback;

pub use depth_2mips_fallback::Depth2MipsFallback;
