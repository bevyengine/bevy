// pathfinder/content/src/fill.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Fill rules.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FillRule {
    Winding,
    EvenOdd,
}
