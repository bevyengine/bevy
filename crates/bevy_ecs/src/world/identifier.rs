use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
// We use usize here because that is the largest `Atomic` we want to require
/// A unique identifier for a [`super::World`].
// Note that this *is* used by external crates as well as for internal safety checks
pub struct WorldId(usize);

/// The next [`WorldId`].
static MAX_WORLD_ID: AtomicUsize = AtomicUsize::new(0);

impl WorldId {
    /// Create a new, unique [`WorldId`]. Returns [`None`] if the supply of unique
    /// [`WorldId`]s has been exhausted
    ///
    /// Please note that the [`WorldId`]s created from this method are unique across
    /// time - if a given [`WorldId`] is [`Drop`]ped its value still cannot be reused
    pub fn new() -> Option<Self> {
        MAX_WORLD_ID
            // We use `Relaxed` here since this atomic only needs to be consistent with itself
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                val.checked_add(1)
            })
            .map(WorldId)
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_ids_unique() {
        let ids = std::iter::repeat_with(WorldId::new)
            .take(50)
            .map(Option::unwrap)
            .collect::<Vec<_>>();
        for (i, &id1) in ids.iter().enumerate() {
            // For the first element, i is 0 - so skip 1
            for &id2 in ids.iter().skip(i + 1) {
                assert_ne!(id1, id2, "WorldIds should not repeat");
            }
        }
    }

    // We cannot use this test as-is, as it causes other tests to panic due to using the same atomic variable.
    // #[test]
    // #[should_panic]
    // fn panic_on_overflow() {
    //     MAX_WORLD_ID.store(usize::MAX - 50, Ordering::Relaxed);
    //     std::iter::repeat_with(WorldId::new)
    //         .take(500)
    //         .for_each(|_| ());
    // }
}
