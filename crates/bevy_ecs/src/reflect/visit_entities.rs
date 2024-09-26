use crate::entity::{Entity, VisitEntities};
use bevy_reflect::{FromReflect, FromType, PartialReflect};

/// For a reflected value, apply an operation to all contained entities.
///
/// See [`VisitEntities`] for more details.
#[derive(Clone)]
pub struct ReflectVisitEntities {
    visit_entities_mut: fn(&mut dyn PartialReflect, &mut dyn FnMut(&mut Entity)),
    visit_entities: fn(&dyn PartialReflect, &mut dyn FnMut(Entity)),
}

impl ReflectVisitEntities {
    /// A general method for applying an operation to all entities in a
    /// reflected component.
    pub fn visit_entities(&self, component: &dyn PartialReflect, f: &mut dyn FnMut(Entity)) {
        (self.visit_entities)(component, f);
    }

    /// A general method for applying an operation that may modify entities in a
    /// reflected component.
    pub fn visit_entities_mut(
        &self,
        component: &mut dyn PartialReflect,
        f: &mut dyn FnMut(&mut Entity),
    ) {
        (self.visit_entities_mut)(component, f);
    }
}

impl<C: FromReflect + VisitEntities> FromType<C> for ReflectVisitEntities {
    fn from_type() -> Self {
        ReflectVisitEntities {
            visit_entities: |component, f| {
                let mut concrete = C::from_reflect(component).unwrap();
                concrete.visit_entities_mut(|entity| f(*entity));
            },
            visit_entities_mut: |component, f| {
                let mut concrete = C::from_reflect(component).unwrap();
                concrete.visit_entities_mut(f);
                component.apply(&concrete);
            },
        }
    }
}
