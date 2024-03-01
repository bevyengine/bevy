//! A module for the [`Gizmos`] [`SystemParam`].

use std::{iter, marker::PhantomData, mem};

use crate::circles::DEFAULT_CIRCLE_SEGMENTS;
use bevy_color::{Color, LinearRgba};
use bevy_ecs::{
    component::Tick,
    system::{Deferred, ReadOnlySystemParam, Res, Resource, SystemBuffer, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_math::{Dir3, Quat, Rotation2d, Vec2, Vec3};
use bevy_transform::TransformPoint;
use bevy_utils::default;

use crate::{
    config::GizmoConfigGroup,
    config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    prelude::GizmoConfig,
};

/// Storage of gizmo primitives.
#[derive(Resource)]
pub struct GizmoStorage<Config, Clear> {
    pub(crate) list_positions: Vec<Vec3>,
    pub(crate) list_colors: Vec<LinearRgba>,
    pub(crate) strip_positions: Vec<Vec3>,
    pub(crate) strip_colors: Vec<LinearRgba>,
    marker: PhantomData<(Config, Clear)>,
}

impl<Config, Clear> Default for GizmoStorage<Config, Clear> {
    fn default() -> Self {
        Self {
            list_positions: default(),
            list_colors: default(),
            strip_positions: default(),
            strip_colors: default(),
            marker: PhantomData,
        }
    }
}

impl<Config, Clear> GizmoStorage<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Combine the other gizmo storage with this one.
    pub fn append_storage<OtherConfig, OtherClear>(
        &mut self,
        other: &GizmoStorage<OtherConfig, OtherClear>,
    ) {
        self.list_positions.extend(other.list_positions.iter());
        self.list_colors.extend(other.list_colors.iter());
        self.strip_positions.extend(other.strip_positions.iter());
        self.strip_colors.extend(other.strip_colors.iter());
    }

    pub(crate) fn swap<OtherConfig, OtherClear>(
        &mut self,
        other: &mut GizmoStorage<OtherConfig, OtherClear>,
    ) {
        mem::swap(&mut self.list_positions, &mut other.list_positions);
        mem::swap(&mut self.list_colors, &mut other.list_colors);
        mem::swap(&mut self.strip_positions, &mut other.strip_positions);
        mem::swap(&mut self.strip_colors, &mut other.strip_colors);
    }

    /// Clear this gizmo storage of any requested gizmos.
    pub fn clear(&mut self) {
        self.list_positions.clear();
        self.list_colors.clear();
        self.strip_positions.clear();
        self.strip_colors.clear();
    }
}

/// Swap buffer for gizmos.
///
/// This is to stash/store the default gizmos so another context can
/// be substituted for that duration.
pub(crate) struct Swap;

/// A [`SystemParam`] for drawing gizmos.
///
/// They are drawn in immediate mode, which means they will be rendered only for
/// the frames, or ticks when in [`FixedMain`](bevy_app::FixedMain), in which
/// they are spawned.
///
/// A system in [`Main`](bevy_app::Main) will be cleared each rendering
/// frame, while a system in [`FixedMain`](bevy_app::FixedMain) will be
/// cleared each time the [`RunFixedMainLoop`](bevy_app::RunFixedMainLoop)
/// schedule is run.
///
/// Gizmos should be spawned before the [`Last`](bevy_app::Last) schedule
/// to ensure they are drawn.
///
/// To set up your own clearing context (useful for custom scheduling similar
/// to [`FixedMain`](bevy_app::FixedMain)):
///
/// ```
/// use bevy_gizmos::{*, gizmos::GizmoStorage};
/// # use bevy_app::prelude::*;
/// # use bevy_ecs::{schedule::ScheduleLabel, prelude::*};
/// # #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
/// # struct StartOfMyContext;
/// # #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
/// # struct EndOfMyContext;
/// # #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
/// # struct StartOfRun;
/// # #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
/// # struct EndOfRun;
/// # struct MyContext;
/// struct ClearContextSetup;
/// impl Plugin for ClearContextSetup {
///     fn build(&self, app: &mut App) {
///         app.init_resource::<GizmoStorage<MyContext>>()
///            .add_systems(StartOfMyContext, stash_default_gizmos)
///            // If not running multiple times, put this with [`stash_default_gizmos`].
///            .add_systems(StartOfRun, clear_gizmo_context::<MyContext>)
///            // If not running multiple times, put this with [`pop_default_gizmos`].
///            .add_systems(EndOfRun, collect_default_gizmos::<MyContext>)
///            .add_systems(EndOfMyContext, pop_default_gizmos)
///            .add_systems(
///                Last,
///                propagate_gizmos::<MyContext>.before(UpdateGizmoMeshes),
///            );
///     }
/// }
/// ```
pub struct Gizmos<'w, 's, Config = DefaultGizmoConfigGroup, Clear = ()>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    buffer: Deferred<'s, GizmoBuffer<Config, Clear>>,
    pub(crate) enabled: bool,
    /// The currently used [`GizmoConfig`]
    pub config: &'w GizmoConfig,
    /// The currently used [`GizmoConfigGroup`]
    pub config_ext: &'w Config,
}

type GizmosState<Config, Clear> = (
    Deferred<'static, GizmoBuffer<Config, Clear>>,
    Res<'static, GizmoConfigStore>,
);
#[doc(hidden)]
pub struct GizmosFetchState<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    state: <GizmosState<Config, Clear> as SystemParam>::State,
}

#[allow(unsafe_code)]
// SAFETY: All methods are delegated to existing `SystemParam` implementations
unsafe impl<Config, Clear> SystemParam for Gizmos<'_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    type State = GizmosFetchState<Config, Clear>;
    type Item<'w, 's> = Gizmos<'w, 's, Config, Clear>;
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        GizmosFetchState {
            state: GizmosState::<Config, Clear>::init_state(world, system_meta),
        }
    }
    fn new_archetype(
        state: &mut Self::State,
        archetype: &bevy_ecs::archetype::Archetype,
        system_meta: &mut SystemMeta,
    ) {
        GizmosState::<Config, Clear>::new_archetype(&mut state.state, archetype, system_meta);
    }
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        GizmosState::<Config, Clear>::apply(&mut state.state, system_meta, world);
    }
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: Delegated to existing `SystemParam` implementations
        let (f0, f1) = unsafe {
            GizmosState::<Config, Clear>::get_param(
                &mut state.state,
                system_meta,
                world,
                change_tick,
            )
        };
        // Accessing the GizmoConfigStore in the immediate mode API reduces performance significantly.
        // Implementing SystemParam manually allows us to do it to here
        // Having config available allows for early returns when gizmos are disabled
        let (config, config_ext) = f1.into_inner().config::<Config>();
        Gizmos {
            buffer: f0,
            enabled: config.enabled,
            config,
            config_ext,
        }
    }
}

#[allow(unsafe_code)]
// Safety: Each field is `ReadOnlySystemParam`, and Gizmos SystemParam does not mutate world
unsafe impl<'w, 's, Config, Clear> ReadOnlySystemParam for Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
    Deferred<'s, GizmoBuffer<Config, Clear>>: ReadOnlySystemParam,
    Res<'w, GizmoConfigStore>: ReadOnlySystemParam,
{
}

struct GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    list_positions: Vec<Vec3>,
    list_colors: Vec<LinearRgba>,
    strip_positions: Vec<Vec3>,
    strip_colors: Vec<LinearRgba>,
    marker: PhantomData<(Config, Clear)>,
}

impl<Config, Clear> Default for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn default() -> Self {
        Self {
            list_positions: default(),
            list_colors: default(),
            strip_positions: default(),
            strip_colors: default(),
            marker: PhantomData,
        }
    }
}

impl<Config, Clear> SystemBuffer for GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let mut storage = world.resource_mut::<GizmoStorage<Config, Clear>>();
        storage.list_positions.append(&mut self.list_positions);
        storage.list_colors.append(&mut self.list_colors);
        storage.strip_positions.append(&mut self.strip_positions);
        storage.strip_colors.append(&mut self.strip_colors);
    }
}

impl<'w, 's, T: GizmoConfigGroup, Clear> Gizmos<'w, 's, T, Clear>
where
    Clear: 'static + Send + Sync,
{
    /// Draw a line in 3D from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line(Vec3::ZERO, Vec3::X, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line(&mut self, start: Vec3, end: Vec3, color: impl Into<Color>) {
        if !self.enabled {
            return;
        }
        self.extend_list_positions([start, end]);
        self.add_list_color(color, 2);
    }

    /// Draw a line in 3D with a color gradient from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::ZERO, Vec3::X, GREEN, RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient<C: Into<Color>>(
        &mut self,
        start: Vec3,
        end: Vec3,
        start_color: C,
        end_color: C,
    ) {
        if !self.enabled {
            return;
        }
        self.extend_list_positions([start, end]);
        self.extend_list_colors([start_color, end_color]);
    }

    /// Draw a line in 3D from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray(Vec3::Y, Vec3::X, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: impl Into<Color>) {
        if !self.enabled {
            return;
        }
        self.line(start, start + vector, color);
    }

    /// Draw a line in 3D with a color gradient from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_gradient(Vec3::Y, Vec3::X, GREEN, RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient<C: Into<Color>>(
        &mut self,
        start: Vec3,
        vector: Vec3,
        start_color: C,
        end_color: C,
    ) {
        if !self.enabled {
            return;
        }
        self.line_gradient(start, start + vector, start_color, end_color);
    }

    /// Draw a line in 3D made of straight segments between the points.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip([Vec3::ZERO, Vec3::X, Vec3::Y], GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip(
        &mut self,
        positions: impl IntoIterator<Item = Vec3>,
        color: impl Into<Color>,
    ) {
        if !self.enabled {
            return;
        }
        self.extend_strip_positions(positions);
        let len = self.buffer.strip_positions.len();
        let linear_color = LinearRgba::from(color.into());
        self.buffer.strip_colors.resize(len - 1, linear_color);
        self.buffer.strip_colors.push(LinearRgba::NAN);
    }

    /// Draw a line in 3D made of straight segments between the points, with a color gradient.
    ///
    /// This should be called for each frame the lines need to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{BLUE, GREEN, RED};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient([
    ///         (Vec3::ZERO, GREEN),
    ///         (Vec3::X, RED),
    ///         (Vec3::Y, BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient<C: Into<Color>>(
        &mut self,
        points: impl IntoIterator<Item = (Vec3, C)>,
    ) {
        if !self.enabled {
            return;
        }
        let points = points.into_iter();

        let GizmoBuffer {
            strip_positions,
            strip_colors,
            ..
        } = &mut *self.buffer;

        let (min, _) = points.size_hint();
        strip_positions.reserve(min);
        strip_colors.reserve(min);

        for (position, color) in points {
            strip_positions.push(position);
            strip_colors.push(LinearRgba::from(color.into()));
        }

        strip_positions.push(Vec3::NAN);
        strip_colors.push(LinearRgba::NAN);
    }

    /// Draw a wireframe sphere in 3D made out of 3 circles around the axes.
    ///
    /// This should be called for each frame the sphere needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::Color;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.sphere(Vec3::ZERO, Quat::IDENTITY, 1., Color::BLACK);
    ///
    ///     // Each circle has 32 line-segments by default.
    ///     // You may want to increase this for larger spheres.
    ///     gizmos
    ///         .sphere(Vec3::ZERO, Quat::IDENTITY, 5., Color::BLACK)
    ///         .circle_segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn sphere(
        &mut self,
        position: Vec3,
        rotation: Quat,
        radius: f32,
        color: impl Into<Color>,
    ) -> SphereBuilder<'_, 'w, 's, T, Clear> {
        SphereBuilder {
            gizmos: self,
            position,
            rotation: rotation.normalize(),
            radius,
            color: color.into(),
            circle_segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a wireframe rectangle in 3D.
    ///
    /// This should be called for each frame the rectangle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect(Vec3::ZERO, Quat::IDENTITY, Vec2::ONE, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect(&mut self, position: Vec3, rotation: Quat, size: Vec2, color: impl Into<Color>) {
        if !self.enabled {
            return;
        }
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2.extend(0.));
        self.linestrip([tl, tr, br, bl, tl], color);
    }

    /// Draw a wireframe cube in 3D.
    ///
    /// This should be called for each frame the cube needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_transform::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cuboid(Transform::IDENTITY, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn cuboid(&mut self, transform: impl TransformPoint, color: impl Into<Color>) {
        let polymorphic_color: Color = color.into();
        if !self.enabled {
            return;
        }
        let rect = rect_inner(Vec2::ONE);
        // Front
        let [tlf, trf, brf, blf] = rect.map(|vec2| transform.transform_point(vec2.extend(0.5)));
        // Back
        let [tlb, trb, brb, blb] = rect.map(|vec2| transform.transform_point(vec2.extend(-0.5)));

        let strip_positions = [
            tlf, trf, brf, blf, tlf, // Front
            tlb, trb, brb, blb, tlb, // Back
        ];
        self.linestrip(strip_positions, polymorphic_color);

        let list_positions = [
            trf, trb, brf, brb, blf, blb, // Front to back
        ];
        self.extend_list_positions(list_positions);

        self.add_list_color(polymorphic_color, 6);
    }

    /// Draw a line in 2D from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_2d(Vec2::ZERO, Vec2::X, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: impl Into<Color>) {
        if !self.enabled {
            return;
        }
        self.line(start.extend(0.), end.extend(0.), color);
    }

    /// Draw a line in 2D with a color gradient from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient_2d(Vec2::ZERO, Vec2::X, GREEN, RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient_2d<C: Into<Color>>(
        &mut self,
        start: Vec2,
        end: Vec2,
        start_color: C,
        end_color: C,
    ) {
        if !self.enabled {
            return;
        }
        self.line_gradient(start.extend(0.), end.extend(0.), start_color, end_color);
    }

    /// Draw a line in 2D made of straight segments between the points.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_2d([Vec2::ZERO, Vec2::X, Vec2::Y], GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_2d(
        &mut self,
        positions: impl IntoIterator<Item = Vec2>,
        color: impl Into<Color>,
    ) {
        if !self.enabled {
            return;
        }
        self.linestrip(positions.into_iter().map(|vec2| vec2.extend(0.)), color);
    }

    /// Draw a line in 2D made of straight segments between the points, with a color gradient.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN, BLUE};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient_2d([
    ///         (Vec2::ZERO, GREEN),
    ///         (Vec2::X, RED),
    ///         (Vec2::Y, BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient_2d<C: Into<Color>>(
        &mut self,
        positions: impl IntoIterator<Item = (Vec2, C)>,
    ) {
        if !self.enabled {
            return;
        }
        self.linestrip_gradient(
            positions
                .into_iter()
                .map(|(vec2, color)| (vec2.extend(0.), color)),
        );
    }

    /// Draw a line in 2D from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_2d(Vec2::Y, Vec2::X, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_2d(&mut self, start: Vec2, vector: Vec2, color: impl Into<Color>) {
        if !self.enabled {
            return;
        }
        self.line_2d(start, start + vector, color);
    }

    /// Draw a line in 2D with a color gradient from `start` to `start + vector`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::Y, Vec3::X, GREEN, RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient_2d<C: Into<Color>>(
        &mut self,
        start: Vec2,
        vector: Vec2,
        start_color: C,
        end_color: C,
    ) {
        if !self.enabled {
            return;
        }
        self.line_gradient_2d(start, start + vector, start_color, end_color);
    }

    /// Draw a wireframe rectangle in 2D.
    ///
    /// This should be called for each frame the rectangle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect_2d(Vec2::ZERO, 0., Vec2::ONE, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect_2d(
        &mut self,
        position: Vec2,
        rotation: impl Into<Rotation2d>,
        size: Vec2,
        color: impl Into<Color>,
    ) {
        if !self.enabled {
            return;
        }
        let rotation: Rotation2d = rotation.into();
        let [tl, tr, br, bl] = rect_inner(size).map(|vec2| position + rotation * vec2);
        self.linestrip_2d([tl, tr, br, bl, tl], color);
    }

    #[inline]
    fn extend_list_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.buffer.list_positions.extend(positions);
    }

    #[inline]
    fn extend_list_colors(&mut self, colors: impl IntoIterator<Item = impl Into<Color>>) {
        self.buffer.list_colors.extend(
            colors
                .into_iter()
                .map(|color| LinearRgba::from(color.into())),
        );
    }

    #[inline]
    fn add_list_color(&mut self, color: impl Into<Color>, count: usize) {
        let polymorphic_color: Color = color.into();
        let linear_color = LinearRgba::from(polymorphic_color);

        self.buffer
            .list_colors
            .extend(iter::repeat(linear_color).take(count));
    }

    #[inline]
    fn extend_strip_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.buffer.strip_positions.extend(positions);
        self.buffer.strip_positions.push(Vec3::NAN);
    }
}

/// A builder returned by [`Gizmos::sphere`].
pub struct SphereBuilder<'a, 'w, 's, Config, Clear = ()>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,
    position: Vec3,
    rotation: Quat,
    radius: f32,
    color: Color,
    circle_segments: usize,
}

impl<Config, Clear> SphereBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments per circle for this sphere.
    pub fn circle_segments(mut self, segments: usize) -> Self {
        self.circle_segments = segments;
        self
    }
}

impl<Config, Clear> Drop for SphereBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }
        for axis in Vec3::AXES {
            self.gizmos
                .circle(
                    self.position,
                    Dir3::new_unchecked(self.rotation * axis),
                    self.radius,
                    self.color,
                )
                .segments(self.circle_segments);
        }
    }
}

fn rect_inner(size: Vec2) -> [Vec2; 4] {
    let half_size = size / 2.;
    let tl = Vec2::new(-half_size.x, half_size.y);
    let tr = Vec2::new(half_size.x, half_size.y);
    let bl = Vec2::new(-half_size.x, -half_size.y);
    let br = Vec2::new(half_size.x, -half_size.y);
    [tl, tr, br, bl]
}
