//! A progress bar widget.
//! Can be used for loading bars, but also health-bars, mana, those kind of things.

use bevy_ecs::{
    prelude::Component,
    query::{Changed, With},
    reflect::ReflectComponent,
    system::Query,
};
use bevy_hierarchy::Children;
use bevy_log::warn;
use bevy_math::map_range;
use bevy_reflect::Reflect;
use bevy_ui::{Size, Style, Val};

/// A progress bar widget.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ProgressBarWidget {
    /// The current progress of the progress bar.
    ///
    /// Valid range between min and max, inclusive.
    progress: f32,
    /// Minimum valid value that progress can have. Inclusive.
    min: f32,
    /// Maximum valid value that progress can have. Inclusive.
    max: f32,
    /// Defines the direction of the `ProgressBarWidget`.
    direction: ProgressBarDirection,
}

/// Defines the direction the progress bar will increase the size of the inner node.
///
/// It increases in the direction of the flex-axis.
#[derive(Default, Debug, Clone, Reflect)]
pub enum ProgressBarDirection {
    /// Direction from FlexStart to FlexEnd
    #[default]
    Horizontal,
    /// Direction from CrossStart to CrossEnd
    Vertical,
}

/// Marker component for the inner box of the progress bar.
#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ProgressBarInner;

impl ProgressBarWidget {
    /// Creates a new [`ProgressBarWidget`].
    pub fn new(progress: f32, min: f32, max: f32) -> Self {
        if min > max {
            panic!("Min should not be larger than max");
        } else {
            ProgressBarWidget {
                progress,
                min,
                max,
                direction: ProgressBarDirection::default(),
            }
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
    mut inner: Query<&mut Style, With<ProgressBarInner>>,
) {
    for (widget, children) in q.iter() {
        for child in children.iter() {
            if let Ok(mut style) = inner.get_mut(*child) {
                let current_size = style.size;
                let new_value = Val::Percent(map_range(
                    widget.get_progress(),
                    (widget.min, widget.max),
                    (0., 100.0),
                ));

                style.size = match widget.direction {
                    ProgressBarDirection::Horizontal => Size::new(new_value, current_size.height),
                    ProgressBarDirection::Vertical => Size::new(current_size.width, new_value),
                };
            }
        }
    }
}
