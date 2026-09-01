use bevy_platform::collections::HashMap;
use core::{hash::Hash, num::NonZeroU32};

/// A free-list allocator for `u32` array indices meant for GPU allocations.
pub struct IndexAllocator {
    free: Vec<u32>,
    high_water_mark: u32,
}

impl IndexAllocator {
    pub fn new() -> Self {
        Self {
            free: Vec::new(),
            high_water_mark: 0,
        }
    }

    /// An exclusive upper bound on every index handed out, for sizing and dispatching over the data
    /// they key.
    ///
    /// Not a live count: released indices leave holes below it, and it never shrinks.
    pub fn high_water_mark(&self) -> u32 {
        self.high_water_mark
    }

    pub fn allocate(&mut self) -> u32 {
        self.free.pop().unwrap_or_else(|| {
            let index = self.high_water_mark;
            self.high_water_mark += 1;
            index
        })
    }

    pub fn release(&mut self, index: u32) {
        self.free.push(index);
    }

    /// How many more indices can be handed out before running past `capacity`.
    fn vacancies(&self, capacity: u32) -> u32 {
        capacity.saturating_sub(self.high_water_mark) + self.free.len() as u32
    }
}

/// Assigns each key an index that stays put for as long as the key is live.
pub struct SlotAllocator<K> {
    slots: HashMap<K, u32>,
    indices: IndexAllocator,
}

impl<K: Eq + Hash> SlotAllocator<K> {
    pub fn new() -> Self {
        Self {
            slots: HashMap::default(),
            indices: IndexAllocator::new(),
        }
    }

    pub fn get(&self, key: &K) -> Option<u32> {
        self.slots.get(key).copied()
    }

    pub fn contains(&self, key: &K) -> bool {
        self.slots.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.slots.keys()
    }

    pub fn get_or_allocate(&mut self, key: K) -> u32 {
        if let Some(&slot) = self.slots.get(&key) {
            return slot;
        }

        let slot = self.indices.allocate();
        self.slots.insert(key, slot);
        slot
    }

    pub fn remove(&mut self, key: &K) -> Option<u32> {
        let slot = self.slots.remove(key)?;
        self.indices.release(slot);
        Some(slot)
    }

    /// How many more distinct keys can be taken on before running past `capacity`.
    fn vacancies(&self, capacity: u32) -> u32 {
        self.indices.vacancies(capacity)
    }
}

/// One occupied slot of a [`RetainedBindingArray`].
struct BindingSlot<T> {
    item: T,
    /// Live references to this slot. An occupied slot always has at least one live reference.
    references: NonZeroU32,
}

/// A binding array whose indices are stable across frames.
///
/// Slots are reference counted by whatever points at them (e.g. materials for textures, instances for
/// mesh slab buffers), and are reused once the last reference goes away.
///
/// `dirty` marks that the contents have changed and the bind group has to be rebuilt.
pub struct RetainedBindingArray<K, T> {
    allocator: SlotAllocator<K>,
    slots: Vec<Option<BindingSlot<T>>>,
    pub dirty: bool,
}

impl<K: Eq + Hash, T> RetainedBindingArray<K, T> {
    pub fn new() -> Self {
        Self {
            allocator: SlotAllocator::new(),
            slots: Vec::new(),
            dirty: false,
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.allocator.contains(key)
    }

    /// How many more distinct keys can be held before running past `capacity`.
    pub fn vacancies(&self, capacity: u32) -> u32 {
        self.allocator.vacancies(capacity)
    }

    /// Whether [`Self::acquire`] would be able to hand out a reference to `key`.
    ///
    /// Callers that need more than one slot at once check this for all of them before acquiring
    /// any, so that they never have to hand a slot straight back.
    pub fn has_room(&self, key: &K, capacity: u32) -> bool {
        self.contains(key) || self.vacancies(capacity) > 0
    }

    /// The array's contents in slot order, with `None` for slots that are currently free.
    pub fn iter(&self) -> impl Iterator<Item = Option<&T>> {
        self.slots
            .iter()
            .map(|slot| slot.as_ref().map(|slot| &slot.item))
    }

    /// Takes a reference to `key`'s slot, allocating and filling it if this is the first one.
    ///
    /// Returns `None` if `key` would need a new slot and every slot below `capacity` is taken. The
    /// bind group layout declares these arrays with a fixed length, so running past it makes
    /// `create_bind_group` fail outright, and callers have to drop whatever wanted the slot.
    pub fn acquire(&mut self, key: K, capacity: u32, item: impl FnOnce() -> T) -> Option<u32> {
        if !self.has_room(&key, capacity) {
            return None;
        }

        let slot = self.allocator.get_or_allocate(key);
        let index = slot as usize;

        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, || None);
        }

        if let Some(occupied) = self.slots[index].as_mut() {
            occupied.references = occupied.references.saturating_add(1);
        } else {
            self.slots[index] = Some(BindingSlot {
                item: item(),
                references: NonZeroU32::MIN,
            });
            self.dirty = true;
        }

        Some(slot)
    }

    /// Drops a reference to `key`'s slot, freeing it if that was the last one.
    pub fn release(&mut self, key: &K) {
        let Some(slot) = self.allocator.get(key) else {
            return;
        };
        let index = slot as usize;
        let Some(occupied) = self.slots[index].as_mut() else {
            return;
        };

        if let Some(remaining) = NonZeroU32::new(occupied.references.get() - 1) {
            occupied.references = remaining;
            return;
        }

        self.slots[index] = None;
        self.allocator.remove(key);
        self.dirty = true;
    }

    /// Repoints an already-allocated slot at a new value, leaving its index and refcount alone.
    pub fn replace(&mut self, key: &K, item: T) {
        if let Some(slot) = self.allocator.get(key)
            && let Some(occupied) = self.slots[slot as usize].as_mut()
        {
            occupied.item = item;
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IndexAllocator, RetainedBindingArray};

    #[test]
    fn index_allocator_reuses_released_slots_without_growing() {
        let mut slots = IndexAllocator::new();

        assert_eq!(slots.allocate(), 0);
        assert_eq!(slots.allocate(), 1);
        assert_eq!(slots.high_water_mark(), 2);

        slots.release(0);
        assert_eq!(slots.allocate(), 0);
        assert_eq!(slots.high_water_mark(), 2);
    }

    #[test]
    fn retained_binding_array_only_dirties_on_binding_changes() {
        let mut bindings = RetainedBindingArray::new();

        assert_eq!(bindings.acquire(7, 2, || 11), Some(0));
        assert!(bindings.dirty);
        bindings.dirty = false;

        // Sharing the existing stable slot only changes its refcount
        assert_eq!(bindings.acquire(7, 2, || 99), Some(0));
        assert!(!bindings.dirty);
        assert_eq!(bindings.iter().next(), Some(Some(&11)));

        bindings.release(&7);
        assert!(!bindings.dirty);
        assert!(bindings.contains(&7));

        // The final release changes the binding array and makes the slot reusable
        bindings.release(&7);
        assert!(bindings.dirty);
        assert!(!bindings.contains(&7));
        assert_eq!(bindings.acquire(8, 2, || 22), Some(0));
    }
}
