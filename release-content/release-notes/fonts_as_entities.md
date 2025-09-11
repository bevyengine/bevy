---
title: Fonts as entities
authors: ["@ickshonpe"]
pull_requests: [20966]
---

The limitations of bevy assets make it difficult to implement responsive font sizing and removal of unused font atlases.
Fonts are now represented by entities with a `Font` component. (todo: Finish this)