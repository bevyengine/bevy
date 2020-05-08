// pathfinder/renderer/src/concurrent/executor.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An abstraction over threading and parallelism systems such as Rayon.

/// An abstraction over threading and parallelism systems such as Rayon.
pub trait Executor {
    /// Like the Rayon snippet:
    ///
    /// ```norun
    /// (0..length).into_par_iter().map(builder).collect()
    /// ```
    fn build_vector<T, F>(&self, length: usize, builder: F) -> Vec<T>
                          where T: Send, F: Fn(usize) -> T + Send + Sync;
}

pub struct SequentialExecutor;

impl Executor for SequentialExecutor {
    fn build_vector<T, F>(&self, length: usize, builder: F) -> Vec<T>
                          where T: Send, F: Fn(usize) -> T + Send + Sync {
        (0..length).into_iter().map(builder).collect()
    }
}
