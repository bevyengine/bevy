---
title: Ui node render extraction is now parallelized.
pull_requests: [22917]
---

`bevy_ui_render` ui node extraction has been parallelized to improve extraction performance.

This has changed how ui node extraction needs to be implemented:
1. `ExtractedUiNodes` is not a resource. Instead use `Local<ExtractedUiNodesAllocator>` in your extraction systems.
   `ExtractedUiNodes` can be retrieved by calling: `ExtractedUiNodesAllocator::allocate`.

   At the end of system execution, `ExtractedUiNodes` need to be queued using
   `ExtractedUiNodesAllocator::queue` method.

   Example:
   ```rust
    fn extract_uinode_example(
        mut commands: Commands,
        mut extracted_uinodes_alloc: Local<ExtractedUiNodesAllocator>,
        // ... 
    ) {
        let mut extracted_uinodes = extracted_uinodes_alloc.allocate();

        // Insert extracted ui nodes into `extracted_uinodes`

        extracted_uinodes_alloc.queue(&mut commands, extracted_uinodes);
    }
   ```

   Queued `ExtractedUiNodes` will be collected and stored inside `ExtractedUiNodesAll` resource in non deterministic order.

2. New field `extended_index` has been added to `TransparentUi`.
   This field can be used to store additional data to correctly identify rendered item,
   similarly to field `index`.
   This field can be set to any value if this additional space is not needed.

