---
title: `System::component_access` has been deleted.
pull_requests: [19496]
---

`System::component_access` has been deleted. If you were calling this method, you can simply use
`my_system.component_access_set().combined_access()` to get the same result.

If you were manually implementing this, it should be equivalent to `System::component_access_set`
anyway.
