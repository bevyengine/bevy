//! Plan playback control.

use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;

use super::schema::ExecutionPlan;

/// Resource for controlling plan playback.
#[derive(Resource, Default, Reflect)]
pub struct PlanPlayback {
    /// Handle to the current plan asset.
    #[reflect(ignore)]
    plan_handle: Option<Handle<ExecutionPlan>>,
    /// Current playback time in seconds.
    current_time: f32,
    /// Whether playback is active.
    is_playing: bool,
    /// Playback speed multiplier.
    speed: f32,
    /// Whether to loop at the end.
    loop_playback: bool,
    /// Total duration of the current plan.
    duration: f32,
    /// Index of the next step to execute.
    next_step_index: usize,
    /// Set of already-executed step indices.
    #[reflect(ignore)]
    executed_steps: bevy_platform::collections::HashSet<usize>,
    /// Whether the plan needs to be reloaded.
    needs_reload: bool,
}

impl PlanPlayback {
    /// Creates a new playback controller.
    pub fn new() -> Self {
        Self {
            speed: 1.0,
            ..Default::default()
        }
    }

    /// Loads a new plan for playback.
    pub fn load_plan(&mut self, handle: Handle<ExecutionPlan>, duration: f32) {
        self.plan_handle = Some(handle);
        self.duration = duration;
        self.reset();
        self.needs_reload = true;
    }

    /// Returns the current plan handle.
    pub fn plan_handle(&self) -> Option<&Handle<ExecutionPlan>> {
        self.plan_handle.as_ref()
    }

    /// Returns whether a reload is needed.
    pub fn needs_reload(&self) -> bool {
        self.needs_reload
    }

    /// Marks the plan as loaded.
    pub fn mark_loaded(&mut self) {
        self.needs_reload = false;
    }

    /// Starts playback.
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Pauses playback.
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Toggles play/pause.
    pub fn toggle(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Returns whether playback is active.
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Seeks to a specific time.
    pub fn seek(&mut self, time: f32) {
        self.current_time = time.clamp(0.0, self.duration);
        // Reset executed steps if seeking backwards
        self.recalculate_executed_steps();
    }

    /// Seeks by a relative amount.
    pub fn seek_relative(&mut self, delta: f32) {
        self.seek(self.current_time + delta);
    }

    /// Resets playback to the beginning.
    pub fn reset(&mut self) {
        self.current_time = 0.0;
        self.next_step_index = 0;
        self.executed_steps.clear();
        self.is_playing = false;
    }

    /// Sets the playback speed.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.0);
    }

    /// Gets the playback speed.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Sets whether to loop.
    pub fn set_loop(&mut self, loop_playback: bool) {
        self.loop_playback = loop_playback;
    }

    /// Returns whether looping is enabled.
    pub fn is_looping(&self) -> bool {
        self.loop_playback
    }

    /// Gets the current playback time.
    pub fn current_time(&self) -> f32 {
        self.current_time
    }

    /// Gets the total duration.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Gets the playback progress (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        if self.duration > 0.0 {
            self.current_time / self.duration
        } else {
            0.0
        }
    }

    /// Advances playback by the given delta time.
    /// Returns true if the end was reached.
    pub fn advance(&mut self, delta_time: f32) -> bool {
        if !self.is_playing {
            return false;
        }

        self.current_time += delta_time * self.speed;

        if self.current_time >= self.duration {
            if self.loop_playback {
                self.current_time = self.current_time % self.duration;
                self.next_step_index = 0;
                self.executed_steps.clear();
            } else {
                self.current_time = self.duration;
                self.is_playing = false;
                return true;
            }
        }

        false
    }

    /// Gets the next step index.
    pub fn next_step_index(&self) -> usize {
        self.next_step_index
    }

    /// Marks a step as executed.
    pub fn mark_step_executed(&mut self, index: usize) {
        self.executed_steps.insert(index);
        if index >= self.next_step_index {
            self.next_step_index = index + 1;
        }
    }

    /// Returns whether a step has been executed.
    pub fn is_step_executed(&self, index: usize) -> bool {
        self.executed_steps.contains(&index)
    }

    /// Recalculates which steps should be marked as executed based on current time.
    fn recalculate_executed_steps(&mut self) {
        // This would need access to the plan to determine which steps
        // have timestamps before current_time
        // For now, just clear and let the executor re-run
        self.executed_steps.clear();
        self.next_step_index = 0;
    }
}
