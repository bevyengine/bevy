---
title: "UncachedQueries"
authors: ["@cbournhonesque"]
pull_requests: [21607]
---

`Query` and `QueryState` now have a third generic parameter `C: QueryCache` that specify how the query caches
the archetypes/tables that match it.

There are two types that implement the `QueryCache` trait:
- `CacheState`: this matches the current behaviour that `Queries` have where each matched archetyped is cached
- `Uncached`: this won't perform any caching, so any query will just query the world from scratch. 


This can be useful for one-off queries where there is no need to catch the list of matched archetypes since it won't be re-used.
A type alias `UncachedQuery = Query<D, F, Uncached>` is provided for convenience.

The following new methods on `World` are introduced:
- `query_uncached<D>`
- `query_filtered_uncached<D, F>`
- `try_query_uncached<D>`
- `try_query_filtered_uncached<D, F>`