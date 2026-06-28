//! Provides a least-recently-used cache container.
use alloc::vec::Vec;
use bevy_platform::collections::HashMap;

/// Index type for node references in the array
type NodeIndex = usize;

/// Special value indicating no node (null pointer equivalent)
const NONE: NodeIndex = usize::MAX;

/// Node structure stored in the flat array
#[derive(Clone)]
struct Node<K, V> {
    key: Option<K>,
    value: Option<V>,
    size: usize,
    prev: NodeIndex,
    next: NodeIndex,
}

impl<K, V> Node<K, V> {
    /// Create an empty node for the free list
    fn empty() -> Self {
        Node {
            key: None,
            value: None,
            size: 0,
            prev: NONE,
            next: NONE,
        }
    }
}

/// A "least recently used" cache. This implements a maplike collection which also remembers
/// the order in which entries were accessed, along with the "size" of each entry. The entry
/// size can be used as a proxy for resources consumed.
///
/// Entries can be evicted in the cache in two ways:
/// * The single least-recently-used entry can be removed singly
/// * Or, entries can be removed until the total size of the cache is below some
///   threshold.
///
/// # Example
/// ```
/// use bevy_utils::LRUCache;
/// let mut cache = LRUCache::new(100);
/// cache.put("small_file", vec![1, 2, 3], 3);
/// cache.put("medium_file", vec![1; 50], 50);
/// cache.put("large_file", vec![1; 30], 30);
///
/// println!("\nCache stats: {:?}", cache.stats());
///
/// // Access items
/// println!(
///     "\nAccessing 'small_file': {:?}",
///     cache.get(&"small_file").map(|v| v.len())
/// );
///
/// cache.evict_while_oversize(100);
/// println!("Cache stats after eviction: {:?}", cache.stats());
/// ```
pub struct LRUCache<K, V>
where
    K: Eq + core::hash::Hash + Clone,
    V: Clone,
{
    /// Current total size of all items in the cache
    total_size: usize,

    /// Flat array storing all nodes. We use a [`Vec`] here so that all nodes have the same
    /// lifetime.
    nodes: Vec<Node<K, V>>,

    /// Hash map from keys to node indices
    map: HashMap<K, NodeIndex>,

    /// Index of the most recently used node
    head: NodeIndex,

    /// Index of the least recently used node
    tail: NodeIndex,

    /// Head of the free list (unused nodes)
    free_head: NodeIndex,

    /// Statistics
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl<K, V> LRUCache<K, V>
where
    K: Eq + core::hash::Hash + Clone,
    V: Clone,
{
    /// Create a new cache with the specified initial capacity
    pub fn new(initial_capacity: usize) -> Self {
        let mut nodes = Vec::with_capacity(initial_capacity);

        // Initialize all nodes as empty and link them in the free list
        for i in 0..initial_capacity {
            let mut node = Node::empty();
            if i > 0 {
                node.prev = i - 1;
            }
            if i < initial_capacity - 1 {
                node.next = i + 1;
            }
            nodes.push(node);
        }

        LRUCache {
            total_size: 0,
            nodes,
            map: HashMap::new(),
            head: NONE,
            tail: NONE,
            free_head: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Get a value from the cache
    pub fn get<'a>(&'a mut self, key: &K) -> Option<&'a V> {
        if let Some(&index) = self.map.get(key) {
            self.hits += 1;
            self.move_to_front(index);
            self.nodes[index].value.as_ref()
        } else {
            self.misses += 1;
            None
        }
    }

    /// Get a value from the cache, but don't update LRU order or stats
    pub fn peek<'a>(&'a mut self, key: &K) -> Option<&'a V> {
        if let Some(&index) = self.map.get(key) {
            self.nodes[index].value.as_ref()
        } else {
            None
        }
    }

    /// Insert or update a key-value pair with the given size
    pub fn put(&mut self, key: K, value: V, size: usize) {
        // Check if key already exists
        if let Some(&index) = self.map.get(&key) {
            // Update existing node
            let old_size = self.nodes[index].size;
            self.total_size = self.total_size - old_size + size;

            self.nodes[index].value = Some(value);
            self.nodes[index].size = size;

            self.move_to_front(index);
        } else {
            // Allocate new node
            let node_index = self.allocate_node();

            // Initialize the node
            self.nodes[node_index] = Node {
                key: Some(key.clone()),
                value: Some(value),
                size,
                prev: NONE,
                next: self.head,
            };

            // Update size stats
            self.total_size += size;

            // Add to map
            self.map.insert(key, node_index);

            // Update list pointers
            if self.head != NONE {
                self.nodes[self.head].prev = node_index;
            }
            self.head = node_index;

            if self.tail == NONE {
                self.tail = node_index;
            }
        }
    }

    /// Remove a specific key from the cache
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(index) = self.map.remove(key) {
            let node = &mut self.nodes[index];
            let value = node.value.take();
            let size = node.size;

            self.total_size -= size;
            self.remove_from_list(index);
            self.free_node(index);

            value
        } else {
            None
        }
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        // Reset all nodes to empty state
        let node_count = self.nodes.len();
        for (i, node) in self.nodes.iter_mut().enumerate() {
            *node = Node::empty();
            node.prev = if i > 0 { i - 1 } else { NONE };
            node.next = if i < node_count - 1 { i + 1 } else { NONE };
        }

        // Clear the map
        self.map.clear();

        // Reset state
        self.total_size = 0;
        self.head = NONE;
        self.tail = NONE;
        self.free_head = 0;
    }

    /// Get current cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            total_size: self.total_size,
            num_items: self.map.len(),
        }
    }

    /// Get current size
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get number of items in cache
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Allocate a node from the free list, growing the array if necessary
    fn allocate_node(&mut self) -> NodeIndex {
        if self.free_head == NONE {
            // Grow the array
            let new_index = self.nodes.len();
            let new_capacity = self.nodes.len() * 2;

            // Add new empty nodes to the free list
            for i in new_index..new_capacity {
                let mut node = Node::empty();
                if i > new_index {
                    node.prev = i - 1;
                }
                if i < new_capacity - 1 {
                    node.next = i + 1;
                } else {
                    node.next = NONE;
                }
                self.nodes.push(node);
            }

            self.free_head = new_index;
        }

        // Take a node from the free list
        let index = self.free_head;
        self.free_head = self.nodes[index].next;

        if self.free_head != NONE {
            self.nodes[self.free_head].prev = NONE;
        }

        self.nodes[index].next = NONE;
        self.nodes[index].prev = NONE;

        index
    }

    /// Return a node to the free list
    fn free_node(&mut self, index: NodeIndex) {
        self.nodes[index] = Node::empty();
        self.nodes[index].next = self.free_head;

        if self.free_head != NONE {
            self.nodes[self.free_head].prev = index;
        }

        self.free_head = index;
    }

    /// Move a node to the front of the LRU list
    fn move_to_front(&mut self, index: NodeIndex) {
        if self.head == index {
            return; // Already at front
        }

        // Remove from current position
        self.remove_from_list(index);

        // Add to front
        self.nodes[index].prev = NONE;
        self.nodes[index].next = self.head;

        if self.head != NONE {
            self.nodes[self.head].prev = index;
        }

        self.head = index;

        if self.tail == NONE {
            self.tail = index;
        }
    }

    /// Remove a node from the LRU list (doesn't free it)
    fn remove_from_list(&mut self, index: NodeIndex) {
        let node = &self.nodes[index];
        let prev = node.prev;
        let next = node.next;

        if prev != NONE {
            self.nodes[prev].next = next;
        } else {
            self.head = next;
        }

        if next != NONE {
            self.nodes[next].prev = prev;
        } else {
            self.tail = prev;
        }
    }

    /// Evict nodes until the total size is within the limit
    pub fn evict_while_oversize(&mut self, size_limit: usize) {
        while self.total_size > size_limit && self.tail != NONE {
            self.evict_lru();
        }
    }

    /// Evict the least recently used node
    pub fn evict_lru(&mut self) {
        if self.tail == NONE {
            return;
        }

        let tail_index = self.tail;
        let key = self.nodes[tail_index].key.clone().unwrap();

        self.remove(&key);
        self.evictions += 1;
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Count of cache hits
    pub hits: u64,
    /// Count of cache misses
    pub misses: u64,
    /// Count of evictions
    pub evictions: u64,
    /// Total size of all entries
    pub total_size: usize,
    /// Number of items in the cache
    pub num_items: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut cache = LRUCache::<&str, usize>::new(4);

        // Test insertion
        cache.put("a", 1, 2);
        cache.put("b", 2, 3);
        cache.put("c", 3, 4);

        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.get(&"d"), None);

        assert_eq!(cache.total_size(), 9);
    }

    #[test]
    fn test_size_based_eviction() {
        let mut cache = LRUCache::<&str, usize>::new(4);

        cache.put("a", 1, 4);
        cache.put("b", 2, 4);
        cache.put("c", 3, 4);
        cache.evict_while_oversize(8);

        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), Some(&3));

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
        assert_eq!(stats.total_size, 8);
    }

    #[test]
    fn test_lru_ordering() {
        let mut cache = LRUCache::<&str, usize>::new(4);

        cache.put("a", 1, 3);
        cache.put("b", 2, 3);
        cache.put("c", 3, 3);

        // Access "a" to make it most recently used
        cache.get(&"a");

        cache.put("d", 4, 3);
        // Should evict "b" (least recently used)
        cache.evict_while_oversize(10);

        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn test_update_size() {
        let mut cache = LRUCache::<&str, usize>::new(4);

        cache.put("a", 1, 3);
        cache.put("b", 2, 3);

        // Update "a" with larger size
        cache.put("a", 10, 8);
        cache.evict_while_oversize(8); // This should evict "b"

        assert_eq!(cache.get(&"a"), Some(&10));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.total_size(), 8);
    }

    #[test]
    fn test_array_growth() {
        let mut cache = LRUCache::<i32, i32>::new(2);

        // Add more items than initial capacity
        for i in 0..10 {
            cache.put(i, i * 10, 1);
        }

        // All items should be present
        for i in 0..10 {
            assert_eq!(cache.get(&i), Some(&(i * 10)));
        }

        assert_eq!(cache.len(), 10);
    }
}
