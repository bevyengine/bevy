---
title: "Moved `AssetEvent` is_* methods to is_*_with_id"
pull_requests: [20816]
---

`AssetEvent::is_loaded_with_dependencies` has been moved to `AssetEvent::is_loaded_with_dependencies_and_id`

`AssetEvent::is_added`, `AssetEvent::is_modified`, `AssetEvent::is_removed`, `AssetEvent::is_unused` have been moved to `AssetEvent::is_added_with_id`, `AssetEvent::is_modified_with_id`, `AssetEvent::is_removed_with_id`, `AssetEvent::is_unused_with_id`

