// pathfinder/resources/src/lib.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An abstraction for reading resources.
//!
//! This accomplishes two purposes over just using the filesystem to locate shaders and so forth:
//! 
//! 1. Downstream users of Pathfinder shouldn't be burdened with having to install the resources
//!    alongside their binary.
//! 
//! 2. There may not be a traditional filesystem available, as for example is the case on Android.

use std::io::Error as IOError;

pub mod embedded;
pub mod fs;

pub trait ResourceLoader {
    /// This is deliberately not a `Path`, because these are virtual paths
    /// that do not necessarily correspond to real paths on a filesystem.
    fn slurp(&self, path: &str) -> Result<Vec<u8>, IOError>;
}
