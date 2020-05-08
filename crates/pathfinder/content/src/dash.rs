// pathfinder/content/src/dash.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Line dashing support.

use crate::outline::{Contour, ContourIterFlags, Outline, PushSegmentFlags};
use std::mem;

const EPSILON: f32 = 0.0001;

pub struct OutlineDash<'a> {
    input: &'a Outline,
    output: Outline,
    state: DashState<'a>,
}

impl<'a> OutlineDash<'a> {
    #[inline]
    pub fn new(input: &'a Outline, dashes: &'a [f32], offset: f32) -> OutlineDash<'a> {
        OutlineDash { input, output: Outline::new(), state: DashState::new(dashes, offset) }
    }

    pub fn dash(&mut self) {
        for contour in &self.input.contours {
            ContourDash::new(contour, &mut self.output, &mut self.state).dash()
        }
    }

    pub fn into_outline(mut self) -> Outline {
        if self.state.is_on() {
            self.output.push_contour(self.state.output);
        }
        self.output
    }
}

struct ContourDash<'a, 'b, 'c> {
    input: &'a Contour,
    output: &'b mut Outline,
    state: &'c mut DashState<'a>,
}

impl<'a, 'b, 'c> ContourDash<'a, 'b, 'c> {
    fn new(input: &'a Contour, output: &'b mut Outline, state: &'c mut DashState<'a>)
           -> ContourDash<'a, 'b, 'c> {
        ContourDash { input, output, state }
    }

    fn dash(&mut self) {
        let mut iterator = self.input.iter(ContourIterFlags::empty());
        let mut queued_segment = None;
        loop {
            if queued_segment.is_none() {
                match iterator.next() {
                    None => break,
                    Some(segment) => queued_segment = Some(segment),
                }
            }

            let mut current_segment = queued_segment.take().unwrap();
            let mut distance = self.state.distance_left;

            let t = current_segment.time_for_distance(distance);
            if t < 1.0 {
                let (prev_segment, next_segment) = current_segment.split(t);
                current_segment = prev_segment;
                queued_segment = Some(next_segment);
            } else {
                distance = current_segment.arc_length();
            }

            if self.state.is_on() {
                self.state.output.push_segment(&current_segment, PushSegmentFlags::empty());
            }

            self.state.distance_left -= distance;
            if self.state.distance_left < EPSILON {
                if self.state.is_on() {
                    self.output.push_contour(mem::replace(&mut self.state.output, Contour::new()));
                }

                self.state.current_dash_index += 1;
                if self.state.current_dash_index == self.state.dashes.len() {
                    self.state.current_dash_index = 0;
                }

                self.state.distance_left = self.state.dashes[self.state.current_dash_index];
            }
        }
    }
}

struct DashState<'a> {
    output: Contour,
    dashes: &'a [f32],
    current_dash_index: usize,
    distance_left: f32,
}

impl<'a> DashState<'a> {
    fn new(dashes: &'a [f32], mut offset: f32) -> DashState<'a> {
        let total: f32 = dashes.iter().cloned().sum();
        offset %= total;

        let mut current_dash_index = 0;
        while current_dash_index < dashes.len() {
            let dash = dashes[current_dash_index];
            if offset < dash {
                break;
            }
            offset -= dash;
            current_dash_index += 1;
        }

        DashState {
            output: Contour::new(),
            dashes,
            current_dash_index,
            distance_left: offset,
        }
    }

    #[inline]
    fn is_on(&self) -> bool {
        self.current_dash_index % 2 == 0
    }
}
