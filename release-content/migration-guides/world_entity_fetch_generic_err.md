---
title: "generic `WorldEntityFetch::FetchMutErr`"
pull_requests: [ 20611 ]
---

`WorldEntityFetch` methods `fetch_mut` and `fetch_deferred_mut` now return a generic error
instead of always returning `EntityMutableFetchError`. This way, fetching a single entity
or entities via an `EntityHashSet` express on the type level that this never can return an
`EntityMutableFetchError::AliasedMutability` error.

This also affects the following APIs:
- `World::get_entity_mut`
- `DeferredWorld::get_entity_mut`
- `EntityFetcher::get_mut`
