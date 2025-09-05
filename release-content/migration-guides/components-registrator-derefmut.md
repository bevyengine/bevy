---
title: "`ComponentsRegistrator` no longer implements `DerefMut`"
pull_requests: [14791, 15458, 15269]
---

`ComponentsRegistrator` no longer implements `DerefMut<Target = Components>`, meaning you won't be able to get a `&mut Components` from it. The only two methods on `Components` that took `&mut self` (`any_queued_mut` and `num_queued_mut`) have been reimplemented on `ComponentsRegistrator`, meaning you won't need to migrate them. Other usages of `&mut Components` were unsupported.
