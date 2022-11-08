//! A progress bar widget.
//! Can be used for loading bars, but also health-bars, mana, those kind of things.

use bevy_ecs::{
    prelude::Component,
    query::{With, Changed},
    system::Query,
};
use bevy_hierarchy::Children;
use bevy_log::warn;


use crate::{Size, Style, Val};

#[derive(Component, Default, Clone, Debug)]
pub struct ProgressBarWidget {
    progress: f32,
    min: f32,
    max: f32
}

/// Marker component for the inner box of the loading bar.
#[derive(Component, Default, Clone, Debug)]
pub struct LoadingBarInner;

impl ProgressBarWidget {
    /// Creates a new [`ProgressBarWidget`].
    pub fn new(progress: f32, min: f32, max: f32) -> Self {
        if min > max { 
            panic!("Min should not be larger than max");
        } else {
            ProgressBarWidget { progress, min, max }
        }
    }

    /// Gets the current progress.
    pub fn get_progress(&self) -> f32 {
        self.progress
    }

    /// Sets the current progress.
    ///
    /// Will output warning if trying to set a value outside the valid range.
    pub fn set_progress(&mut self, progress: f32) {
        if progress >= self.min && progress <= self.max {
            self.progress = progress;
        } else {
            warn!("Trying to set progress out of range");
        }
    }
}

pub(crate) fn update_progress_bars(
    q: Query<(&ProgressBarWidget, &Children), Changed<ProgressBarWidget>>,
    mut inner: Query<&mut Style, With<LoadingBarInner>>,
) {
    for (widget, children) in q.iter() {
        for child in children.iter() {
            if let Ok(mut style) = inner.get_mut(*child) {
                style.size = Size::new(
                    Val::Percent(map_range(widget.get_progress(), (widget.min, widget.max),  (0., 100.0))),
                    Val::Percent(100.0),
                );
            }
        }
    }
}

/// Maps a value from one range of values to a new range of values.
fn map_range(value: f32, old_range: (f32, f32), new_range: (f32, f32)) -> f32 {
    (value - old_range.0) / (old_range.1 - old_range.0) * (new_range.1 - new_range.0) + new_range.0
}
