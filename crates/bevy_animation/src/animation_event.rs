//! Traits and types for triggering events from animations.

use core::{any::Any, fmt::Debug};

use bevy_ecs::prelude::*;
use bevy_reflect::{
    prelude::*, utility::NonGenericTypeInfoCell, ApplyError, DynamicTupleStruct, FromType,
    GetTypeRegistration, ReflectFromPtr, ReflectKind, ReflectMut, ReflectOwned, ReflectRef,
    TupleStructFieldIter, TupleStructInfo, TypeInfo, TypeRegistration, Typed, UnnamedField,
};

pub(crate) fn trigger_animation_event(
    player_entity: Entity,
    target_entity: Entity,
    time: f32,
    event: Box<dyn AnimationEvent>,
) -> impl Command {
    move |world: &mut World| {
        event.trigger(player_entity, time, target_entity, world);
    }
}

// TODO: this should have a derive macro
/// An event that can be used with animations.
#[reflect_trait]
pub trait AnimationEvent: Reflect + Send + Sync {
    /// Trigger the event, targeting `entity`.
    fn trigger(&self, player: Entity, time: f32, target: Entity, world: &mut World);

    /// Clone this value into a new `Box<dyn AnimationEvent>`
    fn clone_value(&self) -> Box<dyn AnimationEvent>;
}

/// The data that will be used to trigger an animation event.
#[derive(TypePath)]
pub(crate) struct AnimationEventData(pub(crate) Box<dyn AnimationEvent>);

impl AnimationEventData {
    pub(crate) fn new(event: impl AnimationEvent) -> Self {
        Self(Box::new(event))
    }
}

impl Debug for AnimationEventData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AnimationEventData(")?;
        PartialReflect::debug(self.0.as_ref(), f)?;
        f.write_str(")")?;
        Ok(())
    }
}

impl Clone for AnimationEventData {
    fn clone(&self) -> Self {
        Self(AnimationEvent::clone_value(self.0.as_ref()))
    }
}

// We have to implement `GetTypeRegistration` manually because of the embedded
// `Box<dyn AnimationEvent>`, which can't be automatically derived yet.
impl GetTypeRegistration for AnimationEventData {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

// We have to implement `Typed` manually because of the embedded
// `Box<dyn AnimationEvent>`, which can't be automatically derived yet.
impl Typed for AnimationEventData {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| {
            TypeInfo::TupleStruct(TupleStructInfo::new::<Self>(&[UnnamedField::new::<()>(0)]))
        })
    }
}

// We have to implement `TupleStruct` manually because of the embedded
// `Box<dyn AnimationEvent>`, which can't be automatically derived yet.
impl TupleStruct for AnimationEventData {
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        match index {
            0 => Some(self.0.as_partial_reflect()),
            _ => None,
        }
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        match index {
            0 => Some(self.0.as_partial_reflect_mut()),
            _ => None,
        }
    }

    fn field_len(&self) -> usize {
        1
    }

    fn iter_fields(&self) -> TupleStructFieldIter {
        TupleStructFieldIter::new(self)
    }

    fn clone_dynamic(&self) -> DynamicTupleStruct {
        DynamicTupleStruct::from_iter([PartialReflect::clone_value(&*self.0)])
    }
}

// We have to implement `PartialReflect` manually because of the embedded
// `Box<dyn AnimationEvent>`, which can't be automatically derived yet.
impl PartialReflect for AnimationEventData {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    #[inline]
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    #[inline]
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    #[inline]
    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    #[inline]
    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let ReflectRef::TupleStruct(struct_value) = value.reflect_ref() {
            for (i, value) in struct_value.iter_fields().enumerate() {
                if let Some(v) = self.field_mut(i) {
                    v.try_apply(value)?;
                }
            }
        } else {
            return Err(ApplyError::MismatchedKinds {
                from_kind: value.reflect_kind(),
                to_kind: ReflectKind::TupleStruct,
            });
        }
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::TupleStruct(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::TupleStruct(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::TupleStruct(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(Clone::clone(self))
    }
}

// We have to implement `Reflect` manually because of the embedded
// `Box<dyn AnimationEvent>`, which can't be automatically derived yet.
impl Reflect for AnimationEventData {
    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

// We have to implement `FromReflect` manually because of the embedded
// `Box<dyn AnimationEvent>`, which can't be automatically derived yet.
impl FromReflect for AnimationEventData {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        Some(reflect.try_downcast_ref::<AnimationEventData>()?.clone())
    }
}
