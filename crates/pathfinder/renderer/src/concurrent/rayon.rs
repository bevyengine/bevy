// pathfinder/renderer/src/concurrent/rayon.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An implementation of the executor using the Rayon library.

use crate::concurrent::executor::Executor;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

pub struct RayonExecutor;

impl Executor for RayonExecutor {
    fn build_vector<T, F>(&self, length: usize, builder: F) -> Vec<T>
                          where T: Send, F: Fn(usize) -> T + Send + Sync {
        (0..length).into_par_iter().map(builder).collect()
    }
}
