// pathfinder/demo/common/src/concurrent.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Concurrency implementation for the demo.

use pathfinder_renderer::concurrent::executor::{Executor, SequentialExecutor};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use rayon::ThreadPoolBuilder;

pub struct DemoExecutor {
    sequential_mode: bool,
}

impl DemoExecutor {
    pub fn new(thread_count: Option<usize>) -> DemoExecutor {
        let sequential_mode = thread_count == Some(1);
        if !sequential_mode {
            let mut thread_pool_builder = ThreadPoolBuilder::new();
            if let Some(thread_count) = thread_count {
                thread_pool_builder = thread_pool_builder.num_threads(thread_count);
            }
            thread_pool_builder.build_global().unwrap();
        }

        DemoExecutor { sequential_mode }
    }
}

impl Executor for DemoExecutor {
    fn build_vector<T, F>(&self, length: usize, builder: F) -> Vec<T>
                          where T: Send, F: Fn(usize) -> T + Send + Sync {
        if self.sequential_mode {
            SequentialExecutor.build_vector(length, builder)
        } else {
            RayonExecutor.build_vector(length, builder)
        }
    }
}
