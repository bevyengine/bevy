---
title: "`take_extract` now returns `dyn FnMut` instead of `dyn Fn`"
pull_requests: [19926]
---

Previously, `set_extract` accepted any `Fn`. Now we accept any `FnMut`. For callers of
`set_extract`, there is no difference since `Fn: FnMut`.

However, callers of `take_extract` will now be returned
`Option<Box<dyn FnMut(&mut World, &mut World) + Send>>` instead.
