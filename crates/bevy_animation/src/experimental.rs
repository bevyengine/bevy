use std::num::NonZeroU16;

// pub struct EntityMap<'a>(&'a [u16]);

// impl<'a> EntityMap<'a> {
//     pub fn get_entity(index: EntityIndex) {

//     }
// }

///////////////////////////////////////////////////////////////////////////////

// Maybe useful for when animating assets or sparse components tree
pub enum FetchState<T> {
    None,
    Missing,
    Found(T),
}

impl<T> FetchState<T> {
    pub fn fetch_mut<F: Fn() -> Option<T>>(&mut self, fetch_fn: F) -> Option<&mut T> {
        match self {
            FetchState::None => {
                if let Some(t) = fetch_fn() {
                    *self = FetchState::Found(t);
                    if let FetchState::Found(t) = self {
                        Some(t)
                    } else {
                        unreachable!()
                    }
                } else {
                    *self = FetchState::Missing;
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
    pub fn entry(&mut self, name: String) -> PropEntry {
        //
    }

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
