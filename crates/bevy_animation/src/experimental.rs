#![allow(dead_code)]

use bevy_ecs::prelude::*;
use fnv::FnvBuildHasher;
use std::{collections::HashMap, num::NonZeroU16};

use crate::custom::{Curves, CurvesUntyped};

// pub struct EntityMap<'a>(&'a [u16]);

// impl<'a> EntityMap<'a> {
//     pub fn get_entity(index: EntityIndex) {

//     }
// }

///////////////////////////////////////////////////////////////////////////////

pub enum FetchState<T> {
    Pending,
    Missing,
    Found(T),
}

pub struct FetchComponentsMut<'a, T: Component> {
    query: Query<'a, &'a mut T>,
    cache: Vec<FetchState<Mut<'a, T>>>,
}

impl<'a, T: Component> FetchComponentsMut<'a, T> {
    pub fn new(query: Query<'a, &'a mut T>) -> Self {
        Self {
            query,
            cache: vec![],
        }
    }

    pub fn begin<'b>(
        &'b mut self,
        entities: &'b [Option<Entity>],
    ) -> FetchComponentsMutGroup<'a, 'b, T> {
        self.cache.clear();
        self.cache
            .resize_with(entities.len(), || FetchState::Pending);

        FetchComponentsMutGroup {
            entities,
            fetch: self,
        }
    }
}

pub struct FetchComponentsMutGroup<'a, 'b, T: Component> {
    entities: &'b [Option<Entity>],
    fetch: &'b mut FetchComponentsMut<'a, T>,
}

impl<'a, 'b, T: Component> FetchComponentsMutGroup<'a, 'b, T> {
    pub fn get_mut<'c: 'a + 'b>(&'c mut self, entity_index: usize) -> Option<&'c mut Mut<'c, T>> {
        let state = &mut self.fetch.cache[entity_index];
        match state {
            FetchState::Pending => {
                // Try fetch once only
                // SAFETY: Only one component will be fetched at the time
                if let Some(t) = self.entities[entity_index]
                    .map(|entity| unsafe { self.fetch.query.get_unsafe(entity).ok() })
                    .flatten()
                {
                    *state = FetchState::Found(t);

                    // TODO: Figure out a nicer way of getting t
                    if let FetchState::Found(t) = state {
                        Some(t)
                    } else {
                        unreachable!()
                    }
                } else {
                    *state = FetchState::Missing;
                    None
                }
            }
            FetchState::Missing => None,
            FetchState::Found(t) => Some(t),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct PropTicket(u16, u16);

// Fast cached lookup table for properties names
struct PropertiesStash {
    table: HashMap<String, u16, FnvBuildHasher>,
    ver: NonZeroU16,
    elements: Vec<CurvesUntyped>,
}

impl Default for PropertiesStash {
    fn default() -> Self {
        Self {
            table: HashMap::default(),
            ver: NonZeroU16::new(1).unwrap(),
            elements: vec![],
        }
    }
}

impl PropertiesStash {
    // pub fn entry(&mut self, name: String) -> PropEntry {
    //     //
    //     todo!()
    // }

    pub fn get<T: 'static>(&self, name: &str, ticket: &mut PropTicket) -> Option<&Curves<T>> {
        if ticket.1 != self.ver.get() {
            // Token expired, find the new ticket
            if let Some(index) = self.table.get(name) {
                *ticket = PropTicket(*index, self.ver.get());
            } else {
                // Not found, set ticket invalid and return none
                *ticket = PropTicket(u16::MAX, self.ver.get());
                return None;
            }
        }

        self.elements
            .get(ticket.0 as usize)
            .map(|curve_untyped| {
                let curves = curve_untyped.downcast_ref::<T>();
                if curves.is_none() {
                    // Wrong type, return the invalid ticket
                    *ticket = PropTicket(u16::MAX, self.ver.get());
                }
                curves
            })
            .flatten()
    }
}
