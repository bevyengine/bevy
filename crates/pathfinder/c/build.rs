// pathfinder/c/build.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use cbindgen;
use std::env;
use std::fs;

fn main() {
    fs::create_dir_all("build/include/pathfinder").expect("Failed to create directories!");
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::generate(crate_dir).expect("cbindgen failed!")
                                 .write_to_file("build/include/pathfinder/pathfinder.h");
}
