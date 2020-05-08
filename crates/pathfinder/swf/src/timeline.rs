// pathfinder/swf/src/timeline.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct PlacementInfo {
    symbol_id: u32,
    translate_x: Twips,
    translate_y: Twips,
}

struct Timeline(Vec<Frame>);

impl Timeline {
    fn first(&self) -> &Frame {
        &self.0[0]
    }

    fn last(&self) -> &Frame {
        &self.0[self.0.len() - 1]
    }

    fn first_mut(&mut self) -> &mut Frame {
        &mut self.0[0]
    }

    fn last_mut(&mut self) -> &mut Frame {
        let last = self.0.len() - 1;
        &mut self.0[last]
    }
}

struct Frame {
    duration_frames_initial: u16,
    duration_remaining_frames: u16,
    placements: Vec<PlacementInfo>
}
