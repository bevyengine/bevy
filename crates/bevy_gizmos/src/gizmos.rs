//! A module for the [`Gizmos`] [`SystemParam`].

use std::{iter, marker::PhantomData};

use crate::circles::DEFAULT_CIRCLE_SEGMENTS;
use bevy_color::LinearRgba;
use bevy_ecs::{
    component::Tick,
    system::{Deferred, ReadOnlySystemParam, Res, Resource, SystemBuffer, SystemMeta, SystemParam},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_math::{Dir3, Mat2, Quat, Rotation2d, Vec2, Vec3};
use bevy_render::color::LegacyColor;
use bevy_transform::TransformPoint;

use crate::{
    config::GizmoConfigGroup,
    config::{DefaultGizmoConfigGroup, GizmoConfigStore},
    prelude::GizmoConfig,
};

type PositionItem = [f32; 3];

#[derive(Resource, Default)]
pub(crate) struct GizmoStorage<T: GizmoConfigGroup> {
    pub(crate) list_positions: Vec<PositionItem>,
    pub(crate) list_colors: Vec<LinearRgba>,
    pub(crate) strip_positions: Vec<PositionItem>,
    pub(crate) strip_colors: Vec<LinearRgba>,
    marker: PhantomData<T>,
}

/// A [`SystemParam`] for drawing gizmos.
///
/// They are drawn in immediate mode, which means they will be rendered only for
/// the frames in which they are spawned.
/// Gizmos should be spawned before the [`Last`](bevy_app::Last) schedule to ensure they are drawn.
pub struct Gizmos<'w, 's, T: GizmoConfigGroup = DefaultGizmoConfigGroup> {
    buffer: Deferred<'s, GizmoBuffer<T>>,
    pub(crate) enabled: bool,
    /// The currently used [`GizmoConfig`]
    pub config: &'w GizmoConfig,
    /// The currently used [`GizmoConfigGroup`]
    pub config_ext: &'w T,
}

type GizmosState<T> = (
    Deferred<'static, GizmoBuffer<T>>,
    Res<'static, GizmoConfigStore>,
);
#[doc(hidden)]
pub struct GizmosFetchState<T: GizmoConfigGroup> {
    state: <GizmosState<T> as SystemParam>::State,
}
// SAFETY: All methods are delegated to existing `SystemParam` implementations
unsafe impl<T: GizmoConfigGroup> SystemParam for Gizmos<'_, '_, T> {
    type State = GizmosFetchState<T>;
    type Item<'w, 's> = Gizmos<'w, 's, T>;
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        GizmosFetchState {
            state: GizmosState::<T>::init_state(world, system_meta),
        }
    }
    fn new_archetype(
        state: &mut Self::State,
        archetype: &bevy_ecs::archetype::Archetype,
        system_meta: &mut SystemMeta,
    ) {
        GizmosState::<T>::new_archetype(&mut state.state, archetype, system_meta);
    }
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        GizmosState::<T>::apply(&mut state.state, system_meta, world);
    }
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        // SAFETY: Delegated to existing `SystemParam` implementations
        let (f0, f1) = unsafe {
            GizmosState::<T>::get_param(&mut state.state, system_meta, world, change_tick)
        };
        // Accessing the GizmoConfigStore in the immediate mode API reduces performance significantly.
        // Implementing SystemParam manually allows us to do it to here
        // Having config available allows for early returns when gizmos are disabled
        let (config, config_ext) = f1.into_inner().config::<T>();
        Gizmos {
            buffer: f0,
            enabled: config.enabled,
            config,
            config_ext,
        }
    }
}
// Safety: Each field is `ReadOnlySystemParam`, and Gizmos SystemParam does not mutate world
unsafe impl<'w, 's, T: GizmoConfigGroup> ReadOnlySystemParam for Gizmos<'w, 's, T>
where
    Deferred<'s, GizmoBuffer<T>>: ReadOnlySystemParam,
    Res<'w, GizmoConfigStore>: ReadOnlySystemParam,
{
}

#[derive(Default)]
struct GizmoBuffer<T: GizmoConfigGroup> {
    list_positions: Vec<PositionItem>,
    list_colors: Vec<LinearRgba>,
    strip_positions: Vec<PositionItem>,
    strip_colors: Vec<LinearRgba>,
    marker: PhantomData<T>,
}

impl<T: GizmoConfigGroup> SystemBuffer for GizmoBuffer<T> {
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let mut storage = world.resource_mut::<GizmoStorage<T>>();
        storage.list_positions.append(&mut self.list_positions);
        storage.list_colors.append(&mut self.list_colors);
        storage.strip_positions.append(&mut self.strip_positions);
        storage.strip_colors.append(&mut self.strip_colors);
    }
}

impl<'w, 's, T: GizmoConfigGroup> Gizmos<'w, 's, T> {
    /// Draw a line in 3D from `start` to `end`.
    ///
    /// This should be called for each frame the line needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line(Vec3::ZERO, Vec3::X, LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line(&mut self, start: Vec3, end: Vec3, color: LegacyColor) {
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::ZERO, Vec3::X, LegacyColor::GREEN, LegacyColor::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient(
        &mut self,
        start: Vec3,
        end: Vec3,
        start_color: LegacyColor,
        end_color: LegacyColor,
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray(Vec3::Y, Vec3::X, LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray(&mut self, start: Vec3, vector: Vec3, color: LegacyColor) {
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_gradient(Vec3::Y, Vec3::X, LegacyColor::GREEN, LegacyColor::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient(
        &mut self,
        start: Vec3,
        vector: Vec3,
        start_color: LegacyColor,
        end_color: LegacyColor,
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip([Vec3::ZERO, Vec3::X, Vec3::Y], LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip(&mut self, positions: impl IntoIterator<Item = Vec3>, color: LegacyColor) {
        if !self.enabled {
            return;
        }
        self.extend_strip_positions(positions);
        let len = self.buffer.strip_positions.len();
        self.buffer.strip_colors.resize(len - 1, color.into());
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient([
    ///         (Vec3::ZERO, LegacyColor::GREEN),
    ///         (Vec3::X, LegacyColor::RED),
    ///         (Vec3::Y, LegacyColor::BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient(&mut self, points: impl IntoIterator<Item = (Vec3, LegacyColor)>) {
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
            strip_positions.push(position.to_array());
            strip_colors.push(color.into());
        }

        strip_positions.push([f32::NAN; 3]);
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.sphere(Vec3::ZERO, Quat::IDENTITY, 1., LegacyColor::BLACK);
    ///
    ///     // Each circle has 32 line-segments by default.
    ///     // You may want to increase this for larger spheres.
    ///     gizmos
    ///         .sphere(Vec3::ZERO, Quat::IDENTITY, 5., LegacyColor::BLACK)
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
        color: LegacyColor,
    ) -> SphereBuilder<'_, 'w, 's, T> {
        SphereBuilder {
            gizmos: self,
            position,
            rotation,
            radius,
            color,
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect(Vec3::ZERO, Quat::IDENTITY, Vec2::ONE, LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect(&mut self, position: Vec3, rotation: Quat, size: Vec2, color: LegacyColor) {
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cuboid(Transform::IDENTITY, LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn cuboid(&mut self, transform: impl TransformPoint, color: LegacyColor) {
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
        self.linestrip(strip_positions, color);

        let list_positions = [
            trf, trb, brf, brb, blf, blb, // Front to back
        ];
        self.extend_list_positions(list_positions);
        self.add_list_color(color, 6);
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_2d(Vec2::ZERO, Vec2::X, LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_2d(&mut self, start: Vec2, end: Vec2, color: LegacyColor) {
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient_2d(Vec2::ZERO, Vec2::X, LegacyColor::GREEN, LegacyColor::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn line_gradient_2d(
        &mut self,
        start: Vec2,
        end: Vec2,
        start_color: LegacyColor,
        end_color: LegacyColor,
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_2d([Vec2::ZERO, Vec2::X, Vec2::Y], LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_2d(&mut self, positions: impl IntoIterator<Item = Vec2>, color: LegacyColor) {
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.linestrip_gradient_2d([
    ///         (Vec2::ZERO, LegacyColor::GREEN),
    ///         (Vec2::X, LegacyColor::RED),
    ///         (Vec2::Y, LegacyColor::BLUE)
    ///     ]);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn linestrip_gradient_2d(
        &mut self,
        positions: impl IntoIterator<Item = (Vec2, LegacyColor)>,
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ray_2d(Vec2::Y, Vec2::X, LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_2d(&mut self, start: Vec2, vector: Vec2, color: LegacyColor) {
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.line_gradient(Vec3::Y, Vec3::X, LegacyColor::GREEN, LegacyColor::RED);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ray_gradient_2d(
        &mut self,
        start: Vec2,
        vector: Vec2,
        start_color: LegacyColor,
        end_color: LegacyColor,
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
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rect_2d(Vec2::ZERO, 0., Vec2::ONE, LegacyColor::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn rect_2d(
        &mut self,
        position: Vec2,
        rotation: impl Into<Rotation2d>,
        size: Vec2,
        color: LegacyColor,
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
        self.buffer
            .list_positions
            .extend(positions.into_iter().map(|vec3| vec3.to_array()));
    }

    #[inline]
    fn extend_list_colors(&mut self, colors: impl IntoIterator<Item = LegacyColor>) {
        self.buffer
            .list_colors
            .extend(colors.into_iter().map(LinearRgba::from));
    }

    #[inline]
    fn add_list_color(&mut self, color: LegacyColor, count: usize) {
        self.buffer
            .list_colors
            .extend(iter::repeat(LinearRgba::from(color)).take(count));
    }

    #[inline]
    fn extend_strip_positions(&mut self, positions: impl IntoIterator<Item = Vec3>) {
        self.buffer.strip_positions.extend(
            positions
                .into_iter()
                .map(|vec3| vec3.to_array())
                .chain(iter::once([f32::NAN; 3])),
        );
    }
}

/// A builder returned by [`Gizmos::sphere`].
pub struct SphereBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    position: Vec3,
    rotation: Quat,
    radius: f32,
    color: LegacyColor,
    circle_segments: usize,
}

impl<T: GizmoConfigGroup> SphereBuilder<'_, '_, '_, T> {
    /// Set the number of line-segments per circle for this sphere.
    pub fn circle_segments(mut self, segments: usize) -> Self {
        self.circle_segments = segments;
        self
    }
}

impl<T: GizmoConfigGroup> Drop for SphereBuilder<'_, '_, '_, T> {
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
