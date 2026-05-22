---
title: Resources as Components
authors: ["@Trashtalk217", "@cart", "@SpecificProtagonist"]
pull_requests: [20934, 22910, 22911, 22919, 22930, 23616, 23716, 24077, 24164]
---

Resources and components have always been separate concepts in Bevy's ECS, even though they're fundamentally the same thing: data stored in the world. While the simple `Res<Time>` sugar is nice, the only real distinction is cardinality — a resource is a component of which at most one exists at any time.

That separation has been a persistent source of friction.
Many of our tools for components (like hooks, observers and relations) simply weren't available for resources,
and the engine carried a significant amount of duplicated internal machinery to keep the two mechanisms in sync.

In Bevy 0.19, resources are now stored as components on singleton entities,
unifying our internals.

You can now:

- Simplify networking and dev-tools code by assuming that entities + components are the only form of data you need to worry about
- Query over both resources and components to support flexible usage patterns
- Add relationships pointing to resource entities
- Add additional components to your resource entities
- Add lifecycle observers to your resource types
- Add your own hooks to resources
- Mark resources as immutable

We don't intend to ever support:

- Changing the storage type of resources.
  - Resources have consistent insertion and access patterns: this is not a useful performance lever to expose.
