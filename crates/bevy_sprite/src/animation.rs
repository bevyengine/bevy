//! Animation Example
//! ```rust
//! use bevy::prelude::*;
//! use bevy::sprite::*;
//!
//! fn main() {
//!     App::build()
//!         .add_plugins(DefaultPlugins)
//!         .add_startup_system(setup.system())
//!         .add_system(bevy::sprite::sprite_animation_system.system())
//!         .run();
//! }
//!
//! fn setup(
//!     mut commands: Commands,
//!     asset_server: Res<AssetServer>,
//!     mut texture_atlases: ResMut<Assets<TextureAtlas>>,
//! ) {
//!     use bevy_sprite::SpriteAnimationBundle;
//! let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
//!     let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1);
//!     let texture_atlas_handle = texture_atlases.add(texture_atlas);
//!
//!     commands.spawn_bundle(OrthographicCameraBundle::new_2d());
//!     commands
//!         .spawn_bundle(SpriteSheetBundle {
//!             texture_atlas: texture_atlas_handle,
//!             transform: Transform::from_scale(Vec3::splat(6.0)),
//!             ..Default::default()
//!         })
//!         .with_bundle(SpriteAnimationBundle{
//!             animations: SpriteAnimations {
//!                 animations: vec![SpriteAnimationBuilder::build(texture_atlas_handle.clone(), 0, 0.2)
//!                     .set_atlas_index(0).add_frame()
//!                     .set_atlas_index(1).add_frame()
//!                     .set_atlas_index(2).add_frame()
//!                     .set_atlas_index(3).add_frame()
//!                     .set_atlas_index(4).add_frame()
//!                     .set_atlas_index(5).add_frame()
//!                     .set_atlas_index(6).add_frame()
//!                     .finish(),
//!                 ]//!
//!             },
//!             ..Default::default()
//!         });
//! }
//! ```
//!

use crate::{TextureAtlas, TextureAtlasSprite, QUAD_HANDLE, SPRITE_SHEET_PIPELINE_HANDLE};
use bevy_asset::Handle;
use bevy_core::{Time, Timer};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::system::{Query, Res};
use bevy_render::draw::{Draw, Visible};
use bevy_render::mesh::Mesh;
use bevy_render::pipeline::{RenderPipeline, RenderPipelines};
use bevy_render::render_graph::base::MainPass;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_utils::tracing::{event, Level};
use bevy_utils::{Duration, HashMap};

/// A Bundle of components for drawing sprite animations from a sprite sheet (also referred to as
/// a `TextureAtlas`)
#[derive(Bundle, Clone)]
pub struct AnimatedSpriteSheetBundle {
    ///
    pub animator: SpriteAnimator,
    /// The specific sprite from the texture atlas to be drawn
    pub sprite: TextureAtlasSprite,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
    pub mesh: Handle<Mesh>,
    // TODO: maybe abstract this out
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for AnimatedSpriteSheetBundle {
    fn default() -> Self {
        Self {
            animator: SpriteAnimator::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                SPRITE_SHEET_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            mesh: QUAD_HANDLE.typed(),
            draw: Default::default(),
            sprite: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

/// A Collection of Sprite Animations and state
#[derive(Clone)]
pub struct SpriteAnimator {
    /// Current Sprite Collection Index
    pub current_collection: Option<usize>,
    /// Current Sprite Animation Index
    pub current_animation: Option<usize>,
    /// Current Sprite Animation Frame
    pub current_frame: Option<usize>,
    /// Current Animation Timer
    pub sprite_timer: Timer,
}

impl SpriteAnimator {
    /// Set the Collection this Animator is currently using.
    pub fn set_collection(&mut self, collection: usize) -> &mut Self {
        self.current_collection = Some(collection);
        self.current_animation = None;
        self.current_frame = None;
        self
    }
    /// Set the Animation this Animator is currently using.
    pub fn set_animation(&mut self, animation: usize) -> &mut Self {
        self.current_animation = Some(animation);
        self.current_frame = None;
        self
    }
}

impl Default for SpriteAnimator {
    fn default() -> SpriteAnimator {
        SpriteAnimator {
            current_collection: None,
            current_animation: None,
            current_frame: None,
            sprite_timer: Timer::new(Default::default(), false),
        }
    }
}

/// A Collection of Sprite Animations
#[derive(Clone)]
pub struct SpriteCatalog {
    name_lookup: HashMap<String, usize>,
    collections: Vec<SpriteCollection>,
}

impl SpriteCatalog {
    /// Add a new Sprite Collection to the Catalog
    pub fn add_collection(&mut self, name: &str, collection: SpriteCollection) -> usize {
        let idx = self.collections.len();
        self.collections.push(collection);
        self.name_lookup.insert(name.to_owned(), idx);
        idx
    }
    /// Get SpriteCollection by index
    pub fn get_collection_by_idx(&self, idx: usize) -> Option<&SpriteCollection> {
        self.collections.get(idx)
    }
    /// Get SpriteCollection by name
    pub fn get_collection_by_name(&self, name: &str) -> Option<&SpriteCollection> {
        self.get_collection_by_idx(*self.name_lookup.get(name)?)
    }
}

/// A Collection of Sprite Animations
#[derive(Clone)]
pub struct SpriteCollection {
    name_lookup: HashMap<String, usize>,
    animations: Vec<SpriteAnimation>,
}

impl SpriteCollection {
    /// Add a new Sprite Animation to the Collection
    pub fn add_animation(&mut self, name: &str, animation: SpriteAnimation) -> usize {
        let idx = self.animations.len();
        self.animations.push(animation);
        self.name_lookup.insert(name.to_owned(), idx);
        idx
    }
    /// Get SpriteAnimation by index
    pub fn get_animation_by_idx(&self, idx: usize) -> Option<&SpriteAnimation> {
        self.animations.get(idx)
    }
    /// Get SpriteAnimation by name
    pub fn get_animation_by_name(&self, name: &str) -> Option<&SpriteAnimation> {
        self.get_animation_by_idx(*self.name_lookup.get(name)?)
    }
}

/// A Sequence of Sprite Frames
#[derive(Clone)]
pub struct SpriteAnimation {
    /// Frames of the animation
    frames: Vec<SpriteAnimationFrame>,
    /// Control How Sprite Animation Plays
    mode: SpriteAnimationMode,
    /// Should Sprite Animation Repeat
    repeat: bool,
}

impl SpriteAnimation {
    /// Get Number of Frames in the Animation
    pub fn get_frame_count(&self) -> usize {
        self.frames.len()
    }
    /// Get SpriteAnimation by index
    pub fn get_frame(&self, idx: usize) -> Option<&SpriteAnimationFrame> {
        self.frames.get(idx)
    }
    /// Get SpriteAnimationMode
    pub fn get_mode(&self) -> SpriteAnimationMode {
        self.mode
    }
    /// Does Animation Repeat
    pub fn repeats(&self) -> bool {
        self.repeat
    }
}

/// Method a Sprite Animation will Play
#[derive(Copy, Clone, Debug)]
pub enum SpriteAnimationMode {
    /// Play Animation from 0..len
    Forward,
    /// Play Animation from len..0
    Reverse,
    /// Play Animation from 0..len..0
    PingPong,
}

impl Default for SpriteAnimationMode {
    fn default() -> SpriteAnimationMode {
        SpriteAnimationMode::Forward
    }
}

/// An Individual Sprite Frame
#[derive(Clone)]
pub struct SpriteAnimationFrame {
    /// Handle for Texture Atlas
    atlas_handle: Handle<TextureAtlas>,
    /// Texture Atlas Sprite Index
    atlas_index: u32,
    /// Duration to set for frame timer
    duration: f32,
}

/// A Builder to help create an animation
pub struct SpriteAnimationBuilder {
    frames: Vec<SpriteAnimationFrame>,
    atlas_handle: Handle<TextureAtlas>,
    atlas_index: u32,
    duration: f32,
    repeat: bool,
    mode: SpriteAnimationMode,
}

impl SpriteAnimationBuilder {
    /// Begin building a new Sprite Animation
    pub fn build(atlas_handle: Handle<TextureAtlas>) -> SpriteAnimationBuilder {
        SpriteAnimationBuilder {
            frames: Vec::new(),
            atlas_handle,
            atlas_index: 0,
            duration: 0.1,
            repeat: true,
            mode: SpriteAnimationMode::Forward,
        }
    }
    /// Set current Atlas Handle
    pub fn set_atlas_handle(&mut self, atlas_handle: Handle<TextureAtlas>) -> &mut Self {
        self.atlas_handle = atlas_handle;
        self
    }
    /// Set current Atlas Index
    pub fn set_atlas_index(&mut self, atlas_index: u32) -> &mut Self {
        self.atlas_index = atlas_index;
        self
    }
    /// Set current Frame Duration
    pub fn set_duration(&mut self, duration: f32) -> &mut Self {
        self.duration = duration;
        self
    }
    /// Set Sprite Animation Mode
    pub fn set_mode(&mut self, mode: SpriteAnimationMode) -> &mut Self {
        self.mode = mode;
        self
    }
    /// Control Sprite Animation Repeat
    pub fn set_repeat(&mut self, repeat: bool) -> &mut Self {
        self.repeat = repeat;
        self
    }
    /// Create new frame from current builder state.
    pub fn add_frame(&mut self) -> &mut Self {
        self.frames.push(SpriteAnimationFrame {
            atlas_handle: self.atlas_handle.clone(),
            atlas_index: self.atlas_index,
            duration: self.duration,
        });
        self
    }
    /// Get current count of frames in animation.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    /// Create animation from current frames.
    pub fn finish(self) -> SpriteAnimation {
        SpriteAnimation {
            frames: self.frames,
            mode: self.mode,
            repeat: self.repeat,
        }
    }
}

pub fn sprite_animation_system(
    time: Res<Time>,
    catalog: Res<SpriteCatalog>,
    mut query: Query<(
        &mut SpriteAnimator,
        &mut Handle<TextureAtlas>,
        &mut TextureAtlasSprite,
    )>,
) {
    for (mut animator, mut atlas, mut sprite) in &mut query.iter_mut() {
        animator.sprite_timer.tick(time.delta());
        if let Some(collection_idx) = animator.current_collection {
            if let Some(collection) = catalog.collections.get(collection_idx) {
                if let Some(animation_idx) = animator.current_animation {
                    if let Some(animation) = collection.animations.get(animation_idx) {
                        // Identify next_frame
                        let next_frame = if let Some(frame_idx) = animator.current_frame {
                            // Advance to next Animation Frame based on mode.
                            match animation.mode {
                                SpriteAnimationMode::Forward => {
                                    if frame_idx + 1 < animation.frames.len() {
                                        frame_idx + 1
                                    } else if animation.repeat {
                                        0
                                    } else {
                                        frame_idx
                                    }
                                }
                                SpriteAnimationMode::Reverse => {
                                    if frame_idx > 0 {
                                        frame_idx - 1
                                    } else if animation.repeat {
                                        animation.frames.len() - 1
                                    } else {
                                        frame_idx
                                    }
                                }
                                SpriteAnimationMode::PingPong => {
                                    let next_frame = frame_idx + 1;
                                    if next_frame < animation.frames.len() {
                                        // Going up
                                        next_frame
                                    } else if next_frame < animation.frames.len() * 2 - 1 {
                                        // Going Down
                                        animation.frames.len() * 2 - next_frame - 2
                                    } else {
                                        // Back at 0
                                        if animation.repeat {
                                            0
                                        } else {
                                            frame_idx
                                        }
                                    }
                                }
                            }
                        } else {
                            // Initialize Animation as no frame exists
                            match animation.mode {
                                SpriteAnimationMode::Forward => 0,
                                SpriteAnimationMode::Reverse => animation.frames.len() - 1,
                                SpriteAnimationMode::PingPong => 0,
                            }
                        };
                        if let Some(frame) = animation.frames.get(next_frame) {
                            // Set the actual Sprite Information
                            let duration = Duration::from_secs_f32(frame.duration);
                            animator.sprite_timer.set_duration(duration);
                            animator.current_frame = Some(next_frame);
                            *atlas = frame.atlas_handle.clone();
                            sprite.index = frame.atlas_index;
                        } else {
                            event!(
                                Level::DEBUG,
                                "Frame {:?} not found for animation {:?} in collection {:?}.",
                                animator.current_frame,
                                animator.current_animation,
                                animator.current_collection,
                            );
                        };
                    } else {
                        event!(
                            Level::DEBUG,
                            "Animation {:?} not found for Collection {:?}.",
                            animation_idx,
                            collection_idx
                        );
                    }
                } else {
                    // No animation Selected. Do Nothing
                    continue;
                }
            } else {
                // Missing Collection
                event!(Level::DEBUG, "Collection {:?} not found.", collection_idx);
            }
        } else {
            // No collection Selected. Do Nothing
            continue;
        }
    }
}
