---
title: Remove default implementation of extend_from_iter from RelationshipSourceCollection
pull_requests: [20255]
---

The `extend_from_iter` method in the `RelationshipSourceCollection` trait no longer has a default implementation. If you have implemented a custom relationship source collection, you must now provide your own implementation of this method.

```rust
// Before: method was optional due to default implementation
impl RelationshipSourceCollection for MyCustomCollection {
    // ... other required methods
    // extend_from_iter was automatically provided
}

// After: method is now required
impl RelationshipSourceCollection for MyCustomCollection {
    // ... other required methods
    fn extend_from_iter(&mut self, entities: impl IntoIterator<Item = Entity>) {
        // Use your collection's native extend method if available
        self.extend(entities);
        // Or implement manually if needed:
        // for entity in entities {
        //     self.add(entity);
        // }
    }
}
```
