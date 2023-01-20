use core::slice;

/// Map-like methods for `Vec<(K, V)>`
pub trait VecMap<K: PartialEq + Ord, V> {
    /// Gets an immutable reference to value by key
    fn get_value(&self, key: K) -> Option<&V>;

    /// Gets a mutable reference to value by key
    fn get_value_mut(&mut self, key: K) -> Option<&mut V>;

    /// Gets an immutable reference to value by key and inserts by closure when it's not preset
    fn get_value_or(&mut self, key: K, or: fn() -> V) -> &V;

    /// Gets an immutable reference to value by key and inserts by closure when it's not preset
    fn get_value_or_mut(&mut self, key: K, or: fn() -> V) -> &mut V;

    /// Gets an immutable reference to value by key and inserts the default when it's not preset
    fn get_value_or_default(&mut self, key: K) -> &V
    where
        V: Default,
    {
        self.get_value_or(key, Default::default)
    }

    /// Gets a mutable reference to value by key and inserts the default when it's not preset
    fn get_value_or_default_mut(&mut self, key: K) -> &mut V
    where
        V: Default,
    {
        self.get_value_or_mut(key, Default::default)
    }

    /// Returns `true` if the given key is preset
    fn contains_key(&self, key: K) -> bool;

    /// Gets the index by the key
    fn index_by_key(&self, key: &K) -> Option<usize>;

    /// Removes the entry by the key
    fn remove_by_key(&mut self, key: K) -> Option<V>;

    /// Returns an iterator over the values
    fn values(&self) -> Values<K, V>;
}

impl<K: PartialEq + Ord, V> VecMap<K, V> for Vec<(K, V)> {
    fn get_value(&self, key: K) -> Option<&V> {
        if let Some(index) = self.index_by_key(&key) {
            unsafe { Some(&self.get_unchecked(index).1) }
        } else {
            None
        }
    }

    fn get_value_mut(&mut self, key: K) -> Option<&mut V> {
        if let Some(index) = self.index_by_key(&key) {
            unsafe { Some(&mut self.get_unchecked_mut(index).1) }
        } else {
            None
        }
    }

    fn get_value_or(&mut self, key: K, or: fn() -> V) -> &V {
        if let Some(index) = self.index_by_key(&key) {
            unsafe { &self.get_unchecked(index).1 }
        } else {
            self.push((key, or()));
            unsafe { &self.last().unwrap_unchecked().1 }
        }
    }

    fn get_value_or_mut(&mut self, key: K, or: fn() -> V) -> &mut V {
        if let Some(index) = self.index_by_key(&key) {
            unsafe { &mut self.get_unchecked_mut(index).1 }
        } else {
            self.push((key, or()));
            unsafe { &mut self.last_mut().unwrap_unchecked().1 }
        }
    }

    fn contains_key(&self, key: K) -> bool {
        self.iter().any(|l| l.0 == key)
    }

    #[inline]
    fn index_by_key(&self, key: &K) -> Option<usize> {
        match self.binary_search_by(|l| l.0.cmp(key)) {
            Ok(index) => Some(index),
            _ => None,
        }
    }

    fn remove_by_key(&mut self, key: K) -> Option<V> {
        self.index_by_key(&key).map(|index| self.remove(index).1)
    }

    fn values(&self) -> Values<K, V> {
        Values { inner: self.iter() }
    }
}

/// Iterator over all values in a VecMap
pub struct Values<'s, K: PartialEq, V> {
    inner: slice::Iter<'s, (K, V)>,
}

impl<'s, K: PartialEq, V> Iterator for Values<'s, K, V> {
    type Item = &'s V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|l| &l.1)
    }
}
