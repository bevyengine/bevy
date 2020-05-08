// pathfinder/examples/lottie_basic/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Experimental example for reading Lottie animations. This is very incomplete.

use pathfinder_lottie::Lottie;
use std::env;
use std::fs::File;
use std::io::BufReader;

fn main() {
    let path = env::args().skip(1).next().unwrap();
    let file = BufReader::new(File::open(path).unwrap());
    let lottie = Lottie::from_reader(file).unwrap();
    println!("{:#?}", lottie);
}
