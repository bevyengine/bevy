use bevy_asset::{Asset, Assets, Handle};
use bevy_reflect::TypePath;
use bevy_render::texture::Image;

/// Provides a ClipOverrideBuilder method to Clips.
pub trait ClipOverridable {
    fn overwrite(self) -> ClipOverrideBuilder;
}

/// Encapsulation of a frame that can be an atlas index or a sprite image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameContent {
    Atlas(usize),
    Image(Handle<Image>),
}

impl From<usize> for FrameContent {
    fn from(frame_index: usize) -> FrameContent {
        FrameContent::Atlas(frame_index)
    }
}

impl From<Handle<Image>> for FrameContent {
    fn from(value: Handle<Image>) -> Self {
        FrameContent::Image(value)
    }
}

/// Set of frames that make a clip
#[derive(Clone, Debug)]
pub enum ClipFrames {
    Atlas(Vec<usize>),
    Image(Vec<Handle<Image>>),
}

impl ClipFrames {
    pub fn get(&self, index: usize) -> Option<FrameContent> {
        match self {
            ClipFrames::Atlas(frames) => frames.get(index).map(|&frame| frame.into()),
            ClipFrames::Image(frames) => frames.get(index).map(|frame| frame.clone().into()),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            ClipFrames::Atlas(frames) => frames.len(),
            ClipFrames::Image(frames) => frames.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ClipFrames::Atlas(frames) => frames.is_empty(),
            ClipFrames::Image(frames) => frames.is_empty(),
        }
    }
}

impl<const N: usize> From<[usize; N]> for ClipFrames {
    fn from(value: [usize; N]) -> Self {
        Self::Atlas(Vec::from(value))
    }
}

/// Direction of a clip
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ClipDirection {
    #[default]
    Forward,
    Backward,
}

/// Looping mode of a clip
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ClipLoopMode {
    /// The number of repetitions an animation will run.
    ///
    /// This means a value of 0 will run once, 1 will run twice and so on.
    Repeat(usize),
    /// The animation will loop indefinitely
    Infinite,
}

impl Default for ClipLoopMode {
    fn default() -> Self {
        Self::Repeat(0)
    }
}

#[derive(TypePath, Asset, Clone, Debug)]
pub struct SpriteClip {
    pub clip: ClipFrames,
    pub clip_loop: ClipLoopMode,
    pub fps: usize,
    pub speed: f32,
}

impl SpriteClip {
    pub fn get(&self, index: usize) -> Option<FrameContent> {
        self.clip.get(index)
    }

    pub fn len(&self) -> usize {
        self.clip.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clip.is_empty()
    }
}

impl ClipOverridable for Handle<SpriteClip> {
    fn overwrite(self) -> ClipOverrideBuilder {
        ClipOverrideBuilder::from(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClipOverride {
    pub source: Handle<SpriteClip>,

    pub loop_mode: Option<ClipLoopMode>,
    pub direction: Option<ClipDirection>,
    pub fps: Option<usize>,
    pub speed: Option<f32>,
}

impl ClipOverride {
    pub fn loop_mode(&self, clips: &Assets<SpriteClip>) -> Option<ClipLoopMode> {
        match (&self.loop_mode, clips.get(&self.source)?.clip_loop) {
            (Some(mode), _) => Some(*mode),
            (None, mode) => Some(mode),
        }
    }

    pub fn fps(&self, clips: &Assets<SpriteClip>) -> Option<usize> {
        match (&self.fps, clips.get(&self.source)?.fps) {
            (Some(fps), _) => Some(*fps),
            (None, fps) => Some(fps),
        }
    }
}

impl From<Handle<SpriteClip>> for ClipOverride {
    fn from(source: Handle<SpriteClip>) -> Self {
        Self {
            source,
            fps: None,
            loop_mode: None,
            direction: None,
            speed: None,
        }
    }
}

impl From<ClipOverrideBuilder> for ClipOverride {
    fn from(value: ClipOverrideBuilder) -> Self {
        value.build()
    }
}

impl ClipOverridable for ClipOverride {
    fn overwrite(self) -> ClipOverrideBuilder {
        ClipOverrideBuilder::from(self)
    }
}

pub struct ClipOverrideBuilder {
    clip: ClipOverride,
}

impl ClipOverrideBuilder {
    pub fn repetitions(mut self, count: usize) -> Self {
        self.clip.loop_mode = Some(ClipLoopMode::Repeat(count));
        self
    }

    pub fn direction(mut self, direction: ClipDirection) -> Self {
        self.clip.direction = Some(direction);
        self
    }

    pub fn looped(mut self) -> Self {
        self.clip.loop_mode = Some(ClipLoopMode::Infinite);
        self
    }

    #[allow(unused_mut)]
    pub fn reverse(mut self) -> Self {
        unimplemented!()
    }

    pub fn forward(mut self) -> Self {
        self.clip.direction = Some(ClipDirection::Forward);
        self
    }

    pub fn backward(mut self) -> Self {
        self.clip.direction = Some(ClipDirection::Backward);
        self
    }

    pub fn fps(mut self, fps: usize) -> Self {
        self.clip.fps = Some(fps);
        self
    }

    pub fn build(self) -> ClipOverride {
        self.clip
    }
}

impl From<Handle<SpriteClip>> for ClipOverrideBuilder {
    fn from(value: Handle<SpriteClip>) -> Self {
        ClipOverrideBuilder { clip: value.into() }
    }
}

impl From<ClipOverride> for ClipOverrideBuilder {
    fn from(clip: ClipOverride) -> Self {
        ClipOverrideBuilder { clip }
    }
}
