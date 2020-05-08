// pathfinder/content/src/render_target.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Render targets.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RenderTargetId {
    pub scene: u32,
    pub render_target: u32,
}
