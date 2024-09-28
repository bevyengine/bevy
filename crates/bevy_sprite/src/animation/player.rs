use crate::animation::clip::{ClipDirection, ClipLoopMode, ClipOverride, FrameContent, SpriteClip};
use bevy_asset::{Asset, Assets};
use bevy_ecs::component::Component;
use bevy_reflect::TypePath;
use bevy_utils::default;
use std::ops::Neg;
use std::time::Duration;

/// Determines when the transition between clips will occur
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TransitionMode {
    #[default]
    /// Transition next update
    Immediate,
    /// Transition occurs when the playing clip animation ends.
    AnimationEnd,
}

/// Clip with its transition settings
#[derive(Clone, Debug, PartialEq)]
pub struct ClipTransition {
    pub clip: ClipOverride,
    pub mode: TransitionMode,
}

impl From<ClipOverride> for ClipTransition {
    fn from(clip: ClipOverride) -> Self {
        Self {
            clip,
            mode: TransitionMode::default(),
        }
    }
}
/// Transition settings
pub struct TransitionParameters {
    pub mode: TransitionMode,
    /*
        starting frame
        preserve time
        etc..
    */
}

impl From<TransitionMode> for TransitionParameters {
    fn from(value: TransitionMode) -> Self {
        Self { mode: value }
    }
}

/// Loads a clip from assets and provides final set of settings between the original and the overrides
pub struct LoadedClip<'a> {
    // Original clip
    clip: &'a SpriteClip,
    // Overridden settings
    clip_loop: ClipLoopMode,
    fps: usize,
    speed: f32,
}

impl<'a> LoadedClip<'a> {
    pub fn try_from(clip: &'a ClipOverride, clips: &'a Assets<SpriteClip>) -> Option<Self> {
        let original_clip = clips.get(&clip.source)?;
        let mut speed = clip.speed.unwrap_or(original_clip.speed);
        match clip.direction {
            None => {}
            Some(ClipDirection::Forward) => speed = speed.abs(),
            Some(ClipDirection::Backward) => speed = speed.abs().neg(),
        }
        Some(Self {
            clip: original_clip,
            clip_loop: clip.loop_mode.unwrap_or(original_clip.clip_loop),
            fps: clip.fps.unwrap_or(original_clip.fps),
            speed,
        })
    }

    /// Clip *frame* length
    pub fn len(&self) -> usize {
        self.clip.len()
    }

    /// Returns if the clip has frames.
    pub fn is_empty(&self) -> bool {
        self.clip.is_empty()
    }

    /// Returns the frame of a given index
    pub fn frame(&self, index: usize) -> Option<FrameContent> {
        self.clip.get(index)
    }
}

/// Current clip playing context
#[derive(Clone, Default, Debug, PartialEq)]
pub struct PlaybackContext {
    /// Total time elapsed since the start of the animation
    pub elapsed_time: f32,
    /// Time overstepped from the previous animation
    pub overstep_transition_time: f32,
    /// Determines if the playing clip has finished
    pub clip_finished: bool,
}

/// Sprite animation player component.
///
/// This component runs every frame and updates the relevant images or atlas indices.
/// You can also run it as an asset component [`Handle<SpriteAnimationPlayer>`] to run an animation to multiple entities in sync.
#[derive(Component, Asset, TypePath, Clone, Debug, PartialEq)]
pub struct SpriteAnimationPlayer {
    // Current animation
    /// Playing clip
    pub clip: Option<ClipOverride>,
    /// Playing clip context
    pub context: PlaybackContext,
    // Next animation
    /// Next clip animation
    pub next_clip: Option<ClipTransition>,

    // Cached variables. We do this because almost every action requires accessing the clip.
    // This way we can provide users useful stuff without making everything a method that requires passing Assets<T>.
    /// Cached current frame
    pub current_frame: Option<FrameContent>,

    // Player settings
    /// Determines if the animation player will run
    pub playing: bool,
    /// Determines the speed of the animation player. This is multiplicative with clip speed.
    pub speed: f32,
}

impl Default for SpriteAnimationPlayer {
    fn default() -> Self {
        Self {
            clip: None,
            next_clip: None,
            current_frame: None,
            playing: true,
            context: PlaybackContext::default(),
            speed: 1.0,
        }
    }
}

impl SpriteAnimationPlayer {
    /// Creates a default player from a clip.
    pub fn new(clip: impl Into<ClipOverride>) -> Self {
        Self {
            next_clip: Some(ClipTransition::from(clip.into())),
            ..default()
        }
    }

    /// Immediately starts playing the specified sprite clip.
    pub fn play(&mut self, clip: impl Into<ClipOverride>) {
        self.playing = true;
        self.next_clip = Some(ClipTransition::from(clip.into()));
    }

    /// Starts playing a [`SpriteClip`] with a [`TransitionMode`]
    pub fn play_with_transition(
        &mut self,
        clip: impl Into<ClipOverride>,
        transition: impl Into<TransitionParameters>,
    ) {
        let transition = transition.into();
        self.playing = true;
        self.next_clip = Some(ClipTransition {
            clip: clip.into(),
            mode: transition.mode,
        });
    }

    /// Stops animation and/or transition of the [`SpriteClip`].
    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Resumes animation of the current [`SpriteClip`].
    pub fn resume(&mut self) {
        self.playing = true;
    }

    /// Restarts the playing [`SpriteClip`] from the beginning.
    pub fn restart(&mut self) {
        self.reset_player();
        self.resume();
    }

    /// Evaluates if a transition should occur during the current update
    fn should_transition(&self) -> bool {
        let Some(next_clip) = &self.next_clip else {
            return false;
        };
        match next_clip.mode {
            TransitionMode::Immediate => true,
            TransitionMode::AnimationEnd => self.context.clip_finished,
        }
    }

    /// Reset sprite player *settings*.
    ///
    /// Clips and transitions are left untouched.
    fn reset_player(&mut self) {
        self.context = PlaybackContext::default();
        self.playing = true;
        self.current_frame = None;
    }

    fn transition(&mut self, clips: &Assets<SpriteClip>) {
        let clip_override = &self
            .next_clip
            .as_ref()
            .expect("checked at should_transition")
            .clip;
        let clip =
            LoadedClip::try_from(clip_override, clips).expect("checked at should_transition");

        match self.next_clip.as_ref().unwrap().mode {
            TransitionMode::Immediate => self.reset_player(),
            TransitionMode::AnimationEnd => {
                // We need to preserve the elapsed time to know what frame should be next.
                // Forward clips work on positive seconds and backwards work on negative seconds,
                // because a clip can go from one direction to another we need to transform the sign.
                let next_elapsed_time = self.context.overstep_transition_time.copysign(clip.speed);
                self.reset_player();
                self.context.elapsed_time = next_elapsed_time;
            }
        }

        self.clip = Some(self.next_clip.take().unwrap().clip);
        debug_assert!(self.next_clip.is_none());
    }

    // The algorithm runs in a time-based manner. Duration is accumulated over the animation,
    // and we calculate the appropriate frame based on that.
    fn update(&mut self, delta: Duration, clips: &Assets<SpriteClip>) -> Option<FrameContent> {
        if self.context.clip_finished {
            return None;
        }

        let clip = LoadedClip::try_from(self.clip.as_ref()?, clips).or_else(|| {
            self.current_frame = None;
            None
        })?;
        let speed = clip.speed * self.speed;
        self.context.elapsed_time += delta.as_secs_f32() * speed;
        // Duration of a clip frame
        let frame_time = 1.0 / clip.fps as f32;
        // Index of the animation
        let animation_index = (self.context.elapsed_time / frame_time).floor().abs() as usize;
        // Index in the clip array
        let frame_index = match speed.is_sign_positive() {
            true => animation_index % clip.len(),
            false => (clip.len() - 1) - animation_index % clip.len(),
        };
        let repetitions = animation_index / clip.len();

        let (mut frame, total_frame_index) = match clip.clip_loop {
            ClipLoopMode::Repeat(n) => {
                // Calculate the total amount of frames in the animation.
                // (n+1) because n=0 means run once, 1 twice and so on.
                // We subtract 1 from total frames to move from counting frames to counting frame *indices*.
                let total_frame_index =
                    (((n + 1) * clip.len()) as f32 * frame_time).floor() as usize - 1;
                (clip.frame(frame_index), Some(total_frame_index))
            }
            ClipLoopMode::Infinite => {
                // Remove time equal to the repetitions so we don't eventually overflow
                if repetitions != 0 {
                    self.context.elapsed_time -= ((repetitions * clip.len()) as f32 * frame_time)
                        .copysign(self.context.elapsed_time);
                }
                (clip.frame(frame_index), None)
            }
        };

        // If we are on the final frame or overstepped it, we return the final frame and signal the clip end.
        if let Some(total_frame_index) = total_frame_index {
            if animation_index >= total_frame_index {
                match speed.is_sign_positive() {
                    true => {
                        frame = clip.frame(clip.len() - 1);
                    }
                    false => {
                        frame = clip.frame(0);
                    }
                }
                self.context.clip_finished = true;
                self.context.overstep_transition_time =
                    (animation_index - total_frame_index) as f32 * frame_time;
            }
        }

        frame
    }

    /// Update states and returns the next index
    pub(crate) fn next(
        &mut self,
        delta: Duration,
        clips: &Assets<SpriteClip>,
    ) -> Option<FrameContent> {
        // Early return if player is disabled
        if !self.playing {
            return None;
        }
        // Check and run transition
        if self.should_transition() {
            self.transition(clips);
        }

        // Update and get the next frame
        let mut frame = self.update(delta, clips);

        // If we finished the animation we need to recheck if we should transition to the next animation
        if self.context.clip_finished && self.should_transition() {
            self.transition(clips);
            frame = self.update(Duration::ZERO, clips);
        }

        // Cache and return frame
        self.current_frame = frame.clone();
        frame
    }
}

#[cfg(test)]
mod tests {
    /// You will notice during these tests that we always skip the first `ATLAS_INDEX`, this is because it uses a time-based algorithm
    /// which means that if we did a `FRAME_TIMESTEP` we would return the next frame.
    /// Instead, we will always test it separately at the start of the iteration.
    ///
    /// Tests only cover the atlas index case. We can assume that if they are right for the atlas index, they will be for the images too.
    use super::*;
    use crate::animation::clip::{ClipFrames, ClipOverridable};
    use crate::animation::SpriteAnimationPlugin;
    use bevy_app::App;
    use bevy_asset::{AssetPlugin, DirectAssetAccessExt, Handle};
    use bevy_time::TimePlugin;

    const ATLAS_INDEXES: [usize; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    const FPS: usize = 1;
    // FIXME Cannot calculate correct timestep due to [`Duration::from_secs_f64`] not being const
    // so let's fake it for the time being.
    // See https://github.com/rust-lang/rust/issues/57241, *might* be fixed with rust 1.82
    const FRAME_TIMESTEP: Duration = Duration::from_secs(1);
    const N_LOOP: usize = 10;

    pub struct TestEnvironment {
        pub app: App,
        pub clip: Handle<SpriteClip>,
    }

    impl TestEnvironment {
        pub fn with_direction(mut self, direction: ClipDirection) -> Self {
            let assets = &mut self
                .app
                .world_mut()
                .get_resource_mut::<Assets<SpriteClip>>()
                .unwrap();
            let clip = assets.get_mut(&self.clip).unwrap();
            clip.speed = match direction {
                ClipDirection::Forward => clip.speed.abs(),
                ClipDirection::Backward => clip.speed.abs().neg(),
            };

            self
        }

        pub fn with_loop(mut self, mode: ClipLoopMode) -> Self {
            let assets = &mut self
                .app
                .world_mut()
                .get_resource_mut::<Assets<SpriteClip>>()
                .unwrap();
            let clip = assets.get_mut(&self.clip).unwrap();
            clip.clip_loop = mode;

            self
        }

        pub fn with_speed(mut self, speed: f32) -> Self {
            let assets = &mut self
                .app
                .world_mut()
                .get_resource_mut::<Assets<SpriteClip>>()
                .unwrap();
            let clip = assets.get_mut(&self.clip).unwrap();
            clip.speed = speed;

            self
        }

        pub fn with_fps(mut self, fps: usize) -> Self {
            let assets = &mut self
                .app
                .world_mut()
                .get_resource_mut::<Assets<SpriteClip>>()
                .unwrap();
            let clip = assets.get_mut(&self.clip).unwrap();
            clip.fps = fps;

            self
        }

        pub fn clips(&self) -> &Assets<SpriteClip> {
            self.app
                .world()
                .get_resource::<Assets<SpriteClip>>()
                .unwrap()
        }
    }

    impl Default for TestEnvironment {
        fn default() -> Self {
            let mut app = App::new();
            app.add_plugins(AssetPlugin::default())
                .add_plugins(TimePlugin)
                .add_plugins(SpriteAnimationPlugin);

            let clip = app.world_mut().add_asset(SpriteClip {
                clip: ClipFrames::from(ATLAS_INDEXES),
                fps: FPS,
                clip_loop: ClipLoopMode::default(),
                speed: 1.0,
            });

            Self { app, clip }
        }
    }

    #[test]
    fn play() {
        const TIME: f32 = f32::INFINITY;
        let ctx = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();

        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_none());
        assert_eq!(player.context.elapsed_time, Duration::ZERO.as_secs_f32());
        // Set player variables to random numbers and play an animation
        player.context.elapsed_time = TIME;
        player.playing = false;
        player.play(ctx.clip.clone());
        // State should be preserved except next clip and playing reset to true
        assert_eq!(player.context.elapsed_time, TIME);
        assert!(player.playing);
        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_some());
        // After updating it should reset back
        assert!(player.next(FRAME_TIMESTEP, ctx.clips()).is_some());
        assert!(player.current_frame.is_some());
        assert!(player.next_clip.is_none());
        assert_eq!(player.context.elapsed_time, FRAME_TIMESTEP.as_secs_f32());
    }

    #[test]
    fn stop() {
        let ctx = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_none());
        assert!(player.playing);
        player.play(ctx.clip.clone());
        assert!(player.next(FRAME_TIMESTEP, ctx.clips()).is_some());
        let mut stopped_player = player.clone();
        stopped_player.stop();
        // Stopping a player shouldn't keep working and state should be preserved
        assert!(stopped_player.next(FRAME_TIMESTEP, ctx.clips()).is_none());
        assert_eq!(player.context, stopped_player.context);
        // Play calls should reset the playing flag
        stopped_player.play(ctx.clip.clone());
        assert!(stopped_player.playing);
        // but they should also not run if you explicitly stop the animation transition
        stopped_player.stop();
        assert!(stopped_player.next(FRAME_TIMESTEP, ctx.clips()).is_none());
    }

    #[test]
    fn resume() {
        let ctx = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_none());
        assert!(player.playing);
        player.play(ctx.clip.clone());
        assert!(player.next(FRAME_TIMESTEP, ctx.clips()).is_some());
        let mut player_stopped = player.clone();
        assert_eq!(player, player_stopped);
        player_stopped.stop();
        assert!(!player_stopped.playing);
        player.next(FRAME_TIMESTEP, ctx.clips());
        player_stopped.next(FRAME_TIMESTEP, ctx.clips());
        player_stopped.resume();
        assert_ne!(player, player_stopped);
        player_stopped.next(FRAME_TIMESTEP, ctx.clips());
        assert_eq!(player, player_stopped);
    }

    #[test]
    fn restart() {
        let ctx = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        player.play(ctx.clip.clone());
        assert!(player.next(FRAME_TIMESTEP, ctx.clips()).is_some());
        let mut player_stopped = player.clone();
        player_stopped.stop();
        assert!(!player_stopped.playing);
        assert!(player.next(FRAME_TIMESTEP, ctx.clips()).is_some());
        assert_ne!(player, player_stopped);
        player_stopped.restart();

        // Running twice again should be in the same state as player
        player_stopped.next(FRAME_TIMESTEP, ctx.clips());
        player_stopped.next(FRAME_TIMESTEP, ctx.clips());
        assert_eq!(player, player_stopped);
    }

    #[test]
    fn dropped_clip_ignored() {
        let mut ctx = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();

        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_none());
        assert_eq!(player.context.elapsed_time, Duration::ZERO.as_secs_f32());
        player.play(ctx.clip.clone());
        // State should be preserved except next clip and playing reset to true
        assert!(player.playing);
        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_some());

        // Updating should give us a new frame
        assert!(player.next(FRAME_TIMESTEP, ctx.clips()).is_some());
        assert!(player.current_frame.is_some());
        // Dropping the handle should not panic
        ctx.app
            .world_mut()
            .resource_mut::<Assets<SpriteClip>>()
            .remove(ctx.clip.id());
        assert!(player.next(FRAME_TIMESTEP, ctx.clips()).is_none());
        assert!(player.current_frame.is_none());
    }

    #[test]
    fn forward() {
        let test = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );
        for &i in ATLAS_INDEXES.iter().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_none());
        assert!(player.current_frame.is_none());
    }

    #[test]
    fn forward_overstep() {
        const TIMES_TIMESTEP: u32 = 2;
        let test = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );
        for &i in ATLAS_INDEXES
            .iter()
            .step_by(TIMES_TIMESTEP as usize)
            .skip(1)
        {
            let frame = player
                .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
                .unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        // We should still be missing a frame, overstepping without a transition should return us the very last frame
        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .is_some());
        assert_eq!(
            player.current_frame.as_ref().unwrap(),
            &FrameContent::from(*ATLAS_INDEXES.last().unwrap())
        );

        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .is_none());
    }

    #[test]
    fn backwards() {
        let test = TestEnvironment::default().with_direction(ClipDirection::Backward);
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.last().copied().unwrap())
        );
        for &i in ATLAS_INDEXES.iter().rev().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_none());
        assert!(player.current_frame.is_none());
    }

    #[test]
    fn backwards_overstep() {
        const TIMES_TIMESTEP: u32 = 2;
        let test = TestEnvironment::default().with_direction(ClipDirection::Backward);
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.last().copied().unwrap())
        );
        for &i in ATLAS_INDEXES
            .iter()
            .rev()
            .step_by(TIMES_TIMESTEP as usize)
            .skip(1)
        {
            let frame = player
                .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
                .unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        // We should still be missing a frame, overstepping without a transition should return us the very last frame
        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .is_some());
        assert_eq!(
            player.current_frame.as_ref().unwrap(),
            &FrameContent::from(*ATLAS_INDEXES.first().unwrap())
        );

        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .as_ref()
            .is_none());
    }

    #[test]
    fn loop_n_times() {
        let test = TestEnvironment::default().with_loop(ClipLoopMode::Repeat(N_LOOP - 1));
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        for &i in ATLAS_INDEXES.repeat(N_LOOP).iter().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_none());
        assert!(player.current_frame.is_none());
    }
    #[test]
    fn loop_n_times_backwards() {
        let test = TestEnvironment::default()
            .with_direction(ClipDirection::Backward)
            .with_loop(ClipLoopMode::Repeat(N_LOOP - 1));
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.last().copied().unwrap())
        );

        for &i in ATLAS_INDEXES.repeat(N_LOOP).iter().rev().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_none());
        assert!(player.current_frame.is_none());
    }

    #[test]
    fn loop_n_times_multiple_same_frame() {
        const TIMES_TIMESTEP: u32 = 25;
        let test = TestEnvironment::default().with_loop(ClipLoopMode::Repeat(N_LOOP - 1));
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        for &i in ATLAS_INDEXES
            .repeat(N_LOOP)
            .iter()
            .step_by(TIMES_TIMESTEP as usize)
            .skip(1)
        {
            let frame = player
                .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
                .unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .is_some());
        assert_eq!(
            player.current_frame.as_ref().unwrap(),
            &FrameContent::from(*ATLAS_INDEXES.last().unwrap())
        );

        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .is_none());
    }

    #[test]
    fn loop_n_times_all_same_frame() {
        const TIMES_TIMESTEP: u32 = ((N_LOOP + 1) * 10) as u32;
        let test = TestEnvironment::default().with_loop(ClipLoopMode::Repeat(N_LOOP));
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);

        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .is_some());
        assert_eq!(
            player.current_frame.as_ref().unwrap(),
            &FrameContent::from(*ATLAS_INDEXES.last().unwrap())
        );

        assert!(player
            .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
            .is_none());
    }

    #[test]
    fn loop_infinite() {
        let test = TestEnvironment::default().with_loop(ClipLoopMode::Infinite);
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        for &i in ATLAS_INDEXES.repeat(10).iter().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        // Doing another step should put us back to the start of the loop
        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_some());
        // Elapsed time should reset back to 0 so we don't overflow
        assert_eq!(player.context.elapsed_time, Duration::ZERO.as_secs_f32());
        assert!(player.current_frame.is_some());
    }

    #[test]
    fn loop_infinite_backwards() {
        let test = TestEnvironment::default()
            .with_loop(ClipLoopMode::Infinite)
            .with_direction(ClipDirection::Backward);
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.last().copied().unwrap())
        );

        for &i in ATLAS_INDEXES.repeat(10).iter().rev().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }

        // Doing another step should put us back to the start of the loop
        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_some());
        // Elapsed time should reset back to 0 so we don't overflow
        assert_eq!(player.context.elapsed_time, Duration::ZERO.as_secs_f32());
        assert!(player.current_frame.is_some());
    }

    #[test]
    fn fps() {
        let test = TestEnvironment::default()
            .with_fps(2)
            .with_loop(ClipLoopMode::Infinite);
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        let mut animation_time = Duration::ZERO;
        for &i in ATLAS_INDEXES.iter().step_by(2).skip(1) {
            animation_time += FRAME_TIMESTEP;
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            assert_eq!(player.context.elapsed_time, animation_time.as_secs_f32());
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
    }

    #[test]
    fn speed() {
        const SPEED: f32 = 2.0;
        let test = TestEnvironment::default()
            .with_speed(SPEED)
            .with_loop(ClipLoopMode::Infinite);
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        for (n_elem, &i) in ATLAS_INDEXES
            .iter()
            .step_by(SPEED as usize)
            .enumerate()
            .skip(1)
        {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            assert_eq!(
                player.context.elapsed_time,
                FRAME_TIMESTEP.as_secs_f32() * SPEED * n_elem as f32
            );
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
    }

    #[test]
    fn speed_player() {
        const SPEED: f32 = 2.0;
        let test = TestEnvironment::default().with_loop(ClipLoopMode::Infinite);
        let mut player = SpriteAnimationPlayer {
            speed: SPEED,
            ..default()
        };
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        for (n_elem, &i) in ATLAS_INDEXES
            .iter()
            .step_by(SPEED as usize)
            .enumerate()
            .skip(1)
        {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            assert_eq!(
                player.context.elapsed_time,
                FRAME_TIMESTEP.as_secs_f32() * SPEED * n_elem as f32
            );
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
    }

    #[test]
    fn speed_multiplicative() {
        const SPEED: f32 = 2.0;
        let test = TestEnvironment::default()
            .with_loop(ClipLoopMode::Infinite)
            .with_speed(SPEED);
        let mut player = SpriteAnimationPlayer {
            speed: SPEED,
            ..default()
        };
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        for (n_elem, &i) in ATLAS_INDEXES
            .iter()
            .step_by((SPEED * SPEED) as usize)
            .enumerate()
            .skip(1)
        {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            assert_eq!(
                player.context.elapsed_time,
                FRAME_TIMESTEP.as_secs_f32() * SPEED * SPEED * n_elem as f32
            );
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
    }

    #[test]
    fn fps_and_speed() {
        let test = TestEnvironment::default()
            .with_fps(2)
            .with_speed(2.0)
            .with_loop(ClipLoopMode::Infinite);
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        let mut animation_time = Duration::ZERO;
        for &i in ATLAS_INDEXES.iter().step_by(4).skip(1) {
            animation_time += FRAME_TIMESTEP * 2;
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            assert_eq!(player.context.elapsed_time, animation_time.as_secs_f32());
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
    }

    #[test]
    fn time_accumulates() {
        #[allow(dead_code)]
        const _TIMESTEP_FRACTION: f32 = 2.0;
        let test = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        assert_eq!(player.current_frame, None);
        assert_eq!(player.context.elapsed_time, Duration::ZERO.as_secs_f32());
        assert_eq!(
            player.next(Duration::ZERO, test.clips()).unwrap(),
            FrameContent::Atlas(ATLAS_INDEXES.first().copied().unwrap())
        );

        let mut animation_time = Duration::ZERO;
        // We will run at half the timestep to check that accumulated time is being added
        for _ in ATLAS_INDEXES.iter().skip(1) {
            animation_time += FRAME_TIMESTEP;
            let frame = player.next(FRAME_TIMESTEP, test.clips());
            assert_eq!(player.context.elapsed_time, animation_time.as_secs_f32());
            assert_eq!(player.current_frame, frame);
        }
    }

    #[test]
    fn transition_immediate() {
        let test = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        // Current frame should still be empty since we did not tick yet
        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_some());
        assert!(player.clip.is_none());
        let frame = player.next(FRAME_TIMESTEP, test.clips());
        // Current frame should be updated after a tick
        assert!(frame.is_some());
        assert_eq!(player.current_frame, frame);
        assert!(player.clip.is_some());
        assert!(player.next_clip.is_none());

        // Playing a new clip should delete previous one and start new
        player.play(test.clip.clone().overwrite().backward());
        assert!(player.next_clip.is_some());
        assert!(player.clip.is_some());
        let next_frame = player.next(Duration::ZERO, test.clips()).unwrap();
        assert!(player.clip.is_some());
        assert!(player.next_clip.is_none());
        assert_eq!(
            next_frame,
            FrameContent::Atlas(*ATLAS_INDEXES.last().unwrap())
        )
    }

    #[test]
    fn animation_end_transition() {
        let test = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        // Current frame should still be empty since we did not tick yet
        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_some());
        assert!(player.clip.is_none());
        let frame = player.next(Duration::ZERO, test.clips());
        // Current frame should be updated after a tick
        assert!(frame.is_some());
        assert_eq!(player.current_frame, frame);
        assert!(player.clip.is_some());
        assert!(player.next_clip.is_none());

        player.play_with_transition(
            test.clip.clone().overwrite().backward(),
            TransitionMode::AnimationEnd,
        );
        assert!(player.next_clip.is_some());
        assert!(player.clip.is_some());
        // First clip should run like normal
        for &i in ATLAS_INDEXES.iter().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
        assert!(player.clip.is_some());
        assert!(player.next_clip.is_none());
        // Second clip should run like normal
        for &i in ATLAS_INDEXES.iter().rev().skip(1) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_none())
    }

    #[test]
    fn animation_end_overstep_transition() {
        const TIMES_TIMESTEP: u32 = 2;
        let test = TestEnvironment::default();
        let mut player = SpriteAnimationPlayer::default();
        player.play(test.clip.clone());
        // Current frame should still be empty since we did not tick yet
        assert!(player.current_frame.is_none());
        assert!(player.next_clip.is_some());
        assert!(player.clip.is_none());
        let frame = player.next(Duration::ZERO, test.clips());
        // Current frame should be updated after a tick
        assert!(frame.is_some());
        assert_eq!(player.current_frame, frame);
        assert!(player.clip.is_some());
        assert!(player.next_clip.is_none());

        player.play_with_transition(
            test.clip.clone().overwrite().backward(),
            TransitionMode::AnimationEnd,
        );
        assert!(player.next_clip.is_some());
        assert!(player.clip.is_some());
        // First clip should run like normal
        for &i in ATLAS_INDEXES
            .iter()
            .step_by(TIMES_TIMESTEP as usize)
            .skip(1)
        {
            let frame = player
                .next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips())
                .unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
        // We are still missing a times*timestep so both clips should still be alive
        assert!(player.clip.is_some());
        assert!(player.next_clip.is_some());
        player.next(FRAME_TIMESTEP * TIMES_TIMESTEP, test.clips());
        // We should be on the next clip in frame 8
        assert!(player.clip.is_some());
        assert!(player.next_clip.is_none());
        assert_eq!(
            player.current_frame.as_ref().unwrap(),
            &FrameContent::from(*ATLAS_INDEXES.get(8).unwrap())
        );

        // Second clip should run like normal
        for &i in ATLAS_INDEXES.iter().rev().skip(2) {
            let frame = player.next(FRAME_TIMESTEP, test.clips()).unwrap();
            match frame {
                FrameContent::Atlas(index) => assert_eq!(index, i),
                _ => unreachable!(),
            }
            assert_eq!(player.current_frame.as_ref().expect("should exist"), &frame);
        }
        assert!(player.next(FRAME_TIMESTEP, test.clips()).is_none())
    }
}
