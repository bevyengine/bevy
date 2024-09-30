use crate::entity::{Entity, VisitEntities};
use bevy_reflect::{FromReflect, FromType, PartialReflect};

/// For a reflected value, apply an operation to all contained entities.
///
/// See [`VisitEntities`] for more details.
#[derive(Clone)]
pub struct ReflectVisitEntities {
    visit_entities: fn(&dyn PartialReflect, &mut dyn FnMut(Entity)),
}

impl ReflectVisitEntities {
    /// A general method for applying an operation to all entities in a
    /// reflected component.
    pub fn visit_entities(&self, component: &dyn PartialReflect, f: &mut dyn FnMut(Entity)) {
        (self.visit_entities)(component, f);
    }
}

impl<C: FromReflect + VisitEntities> FromType<C> for ReflectVisitEntities {
    fn from_type() -> Self {
        ReflectVisitEntities {
            visit_entities: |component, f| {
                let concrete = C::from_reflect(component).unwrap();
                concrete.visit_entities(f);
            },
        }
    }
}
