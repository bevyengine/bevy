use crate::{core::Time, prelude::SystemBuilder};
use legion::prelude::Schedulable;
use std::collections::VecDeque;

pub fn build_fps_printer_system() -> Box<dyn Schedulable> {
    let mut elapsed = 0.0;
    let mut frame_time_total = 0.0;
    let mut frame_time_count = 0;
    let frame_time_max = 10;
    let mut frame_time_values = VecDeque::new();
    SystemBuilder::new("FpsPrinter")
        .read_resource::<Time>()
        .build(move |_, _world, time, _queries| {
            elapsed += time.delta_seconds;
            frame_time_values.push_front(time.delta_seconds);
            frame_time_total += time.delta_seconds;
            frame_time_count += 1;
            if frame_time_count > frame_time_max {
                frame_time_count = frame_time_max;
                frame_time_total -= frame_time_values.pop_back().unwrap();
            }
            if elapsed > 1.0 {
                if frame_time_count > 0 && frame_time_total > 0.0 {
                    println!(
                        "fps: {}",
                        1.0 / (frame_time_total / frame_time_count as f32)
                    )
                }
                elapsed = 0.0;
            }
        })
}
