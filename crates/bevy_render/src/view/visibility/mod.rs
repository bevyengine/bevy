use core::any::TypeId;

use bevy_ecs::{component::Component, entity::Entity, prelude::ReflectComponent};

mod range;
use bevy_camera::visibility::*;
pub use range::*;

use crate::sync_world::MainEntity;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::hierarchy::validate_parent_has_component;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::{Parallel, TypeIdMap};
use smallvec::SmallVec;

/// User indication of whether an entity is visible. Propagates down the entity hierarchy.
///
/// If an entity is hidden in this way, all [`Children`] (and all of their children and so on) who
/// are set to [`Inherited`](Self::Inherited) will also be hidden.
///
/// This is done by the `visibility_propagate_system` which uses the entity hierarchy and
/// `Visibility` to set the values of each entity's [`InheritedVisibility`] component.
#[derive(Component, Clone, Copy, Reflect, Debug, PartialEq, Eq, Default)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(InheritedVisibility, ViewVisibility)]
pub enum Visibility {
    /// An entity with `Visibility::Inherited` will inherit the Visibility of its [`ChildOf`] target.
    ///
    /// A root-level entity that is set to `Inherited` will be visible.
    #[default]
    Inherited,
    /// An entity with `Visibility::Hidden` will be unconditionally hidden.
    Hidden,
    /// An entity with `Visibility::Visible` will be unconditionally visible.
    ///
    /// Note that an entity with `Visibility::Visible` will be visible regardless of whether the
    /// [`ChildOf`] target entity is hidden.
    Visible,
}

impl Visibility {
    /// Toggles between `Visibility::Inherited` and `Visibility::Visible`.
    /// If the value is `Visibility::Hidden`, it remains unaffected.
    #[inline]
    pub fn toggle_inherited_visible(&mut self) {
        *self = match *self {
            Visibility::Inherited => Visibility::Visible,
            Visibility::Visible => Visibility::Inherited,
            _ => *self,
        };
    }
    /// Toggles between `Visibility::Inherited` and `Visibility::Hidden`.
    /// If the value is `Visibility::Visible`, it remains unaffected.
    #[inline]
    pub fn toggle_inherited_hidden(&mut self) {
        *self = match *self {
            Visibility::Inherited => Visibility::Hidden,
            Visibility::Hidden => Visibility::Inherited,
            _ => *self,
        };
    }
    /// Toggles between `Visibility::Visible` and `Visibility::Hidden`.
    /// If the value is `Visibility::Inherited`, it remains unaffected.
    #[inline]
    pub fn toggle_visible_hidden(&mut self) {
        *self = match *self {
            Visibility::Visible => Visibility::Hidden,
            Visibility::Hidden => Visibility::Visible,
            _ => *self,
        };
    }
}

// Allows `&Visibility == Visibility`
impl PartialEq<Visibility> for &Visibility {
    #[inline]
    fn eq(&self, other: &Visibility) -> bool {
        // Use the base Visibility == Visibility implementation.
        <Visibility as PartialEq<Visibility>>::eq(*self, other)
    }
}

// Allows `Visibility == &Visibility`
impl PartialEq<&Visibility> for Visibility {
    #[inline]
    fn eq(&self, other: &&Visibility) -> bool {
        // Use the base Visibility == Visibility implementation.
        <Visibility as PartialEq<Visibility>>::eq(self, *other)
    }
}

/// Whether or not an entity is visible in the hierarchy.
/// This will not be accurate until [`VisibilityPropagate`] runs in the [`PostUpdate`] schedule.
///
/// If this is false, then [`ViewVisibility`] should also be false.
///
/// [`VisibilityPropagate`]: VisibilitySystems::VisibilityPropagate
#[derive(Component, Deref, Debug, Default, Clone, Copy, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[component(on_insert = validate_parent_has_component::<Self>)]
pub struct InheritedVisibility(bool);

impl InheritedVisibility {
    /// An entity that is invisible in the hierarchy.
    pub const HIDDEN: Self = Self(false);
    /// An entity that is visible in the hierarchy.
    pub const VISIBLE: Self = Self(true);

    /// Returns `true` if the entity is visible in the hierarchy.
    /// Otherwise, returns `false`.
    #[inline]
    pub fn get(self) -> bool {
        self.0
    }
}

/// A bucket into which we group entities for the purposes of visibility.
///
/// Bevy's various rendering subsystems (3D, 2D, UI, etc.) want to be able to
/// quickly winnow the set of entities to only those that the subsystem is
/// tasked with rendering, to avoid spending time examining irrelevant entities.
/// At the same time, Bevy wants the [`check_visibility`] system to determine
/// all entities' visibilities at the same time, regardless of what rendering
/// subsystem is responsible for drawing them. Additionally, your application
/// may want to add more types of renderable objects that Bevy determines
/// visibility for just as it does for Bevy's built-in objects.
///
/// The solution to this problem is *visibility classes*. A visibility class is
/// a type, typically the type of a component, that represents the subsystem
/// that renders it: for example, `Mesh3d`, `Mesh2d`, and `Sprite`. The
/// [`VisibilityClass`] component stores the visibility class or classes that
/// the entity belongs to. (Generally, an object will belong to only one
/// visibility class, but in rare cases it may belong to multiple.)
///
/// When adding a new renderable component, you'll typically want to write an
/// add-component hook that adds the type ID of that component to the
/// [`VisibilityClass`] array. See `custom_phase_item` for an example.
//
// Note: This can't be a `ComponentId` because the visibility classes are copied
// into the render world, and component IDs are per-world.
#[derive(Clone, Component, Default, Reflect, Deref, DerefMut)]
#[reflect(Component, Default, Clone)]
pub struct VisibilityClass(pub SmallVec<[TypeId; 1]>);

/// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering.
///
/// Each frame, this will be reset to `false` during [`VisibilityPropagate`] systems in [`PostUpdate`].
/// Later in the frame, systems in [`CheckVisibility`] will mark any visible entities using [`ViewVisibility::set`].
/// Because of this, values of this type will be marked as changed every frame, even when they do not change.
///
/// If you wish to add custom visibility system that sets this value, make sure you add it to the [`CheckVisibility`] set.
///
/// [`VisibilityPropagate`]: VisibilitySystems::VisibilityPropagate
/// [`CheckVisibility`]: VisibilitySystems::CheckVisibility
#[derive(Component, Deref, Debug, Default, Clone, Copy, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ViewVisibility(bool);

impl ViewVisibility {
    /// An entity that cannot be seen from any views.
    pub const HIDDEN: Self = Self(false);

    /// Returns `true` if the entity is visible in any view.
    /// Otherwise, returns `false`.
    #[inline]
    pub fn get(self) -> bool {
        self.0
    }

    /// Sets the visibility to `true`. This should not be considered reversible for a given frame,
    /// as this component tracks whether or not the entity visible in _any_ view.
    ///
    /// This will be automatically reset to `false` every frame in [`VisibilityPropagate`] and then set
    /// to the proper value in [`CheckVisibility`].
    ///
    /// You should only manually set this if you are defining a custom visibility system,
    /// in which case the system should be placed in the [`CheckVisibility`] set.
    /// For normal user-defined entity visibility, see [`Visibility`].
    ///
    /// [`VisibilityPropagate`]: VisibilitySystems::VisibilityPropagate
    /// [`CheckVisibility`]: VisibilitySystems::CheckVisibility
    #[inline]
    pub fn set(&mut self) {
        self.0 = true;
    }
}

/// Use this component to opt-out of built-in frustum culling for entities, see
/// [`Frustum`].
///
/// It can be used for example:
/// - when a [`Mesh`] is updated but its [`Aabb`] is not, which might happen with animations,
/// - when using some light effects, like wanting a [`Mesh`] out of the [`Frustum`]
///   to appear in the reflection of a [`Mesh`] within.
#[derive(Debug, Component, Default, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct NoFrustumCulling;

/// Collection of entities visible from the current view.
///
/// This component contains all entities which are visible from the currently
/// rendered view. The collection is updated automatically by the [`VisibilitySystems::CheckVisibility`]
/// system set. Renderers can use the equivalent [`RenderVisibleEntities`] to optimize rendering of
/// a particular view, to prevent drawing items not visible from that view.
///
/// This component is intended to be attached to the same entity as the [`Camera`] and
/// the [`Frustum`] defining the view.
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct VisibleEntities {
    #[reflect(ignore, clone)]
    pub entities: TypeIdMap<Vec<Entity>>,
}

impl VisibleEntities {
    pub fn get(&self, type_id: TypeId) -> &[Entity] {
        match self.entities.get(&type_id) {
            Some(entities) => &entities[..],
            None => &[],
        }
    }

    pub fn get_mut(&mut self, type_id: TypeId) -> &mut Vec<Entity> {
        self.entities.entry(type_id).or_default()
    }

    pub fn iter(&self, type_id: TypeId) -> impl DoubleEndedIterator<Item = &Entity> {
        self.get(type_id).iter()
    }

    pub fn len(&self, type_id: TypeId) -> usize {
        self.get(type_id).len()
    }

    pub fn is_empty(&self, type_id: TypeId) -> bool {
        self.get(type_id).is_empty()
    }

    pub fn clear(&mut self, type_id: TypeId) {
        self.get_mut(type_id).clear();
    }

    pub fn clear_all(&mut self) {
        // Don't just nuke the hash table; we want to reuse allocations.
        for entities in self.entities.values_mut() {
            entities.clear();
        }
    }

    pub fn push(&mut self, entity: Entity, type_id: TypeId) {
        self.get_mut(type_id).push(entity);
    }
}

/// Collection of entities visible from the current view.
///
/// This component is extracted from [`VisibleEntities`].
#[derive(Clone, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct RenderVisibleEntities {
    #[reflect(ignore, clone)]
    pub entities: TypeIdMap<Vec<(Entity, MainEntity)>>,
}

impl RenderVisibleEntities {
    pub fn get<QF>(&self) -> &[(Entity, MainEntity)]
    where
        QF: 'static,
    {
        match self.entities.get(&TypeId::of::<QF>()) {
            Some(entities) => &entities[..],
            None => &[],
        }
    }

    pub fn iter<QF>(&self) -> impl DoubleEndedIterator<Item = &(Entity, MainEntity)>
    where
        QF: 'static,
    {
        self.get::<QF>().iter()
    }

    pub fn len<QF>(&self) -> usize
    where
        QF: 'static,
    {
        self.get::<QF>().len()
    }

    pub fn is_empty<QF>(&self) -> bool
    where
        QF: 'static,
    {
        self.get::<QF>().is_empty()
    }
}
