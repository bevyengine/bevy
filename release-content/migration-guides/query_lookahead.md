---
title: "WorldQuery, QueryData, and QueryFilter trait methods rearranged"
pull_requests: [22500]
---

- `QueryData::IS_ARCHETYPAL`, `QueryFilter::IS_ARCHETYPAL` have been moved to `WorldQuery`.
- `QueryFilter::filter_fetch` has been moved to `WorldQuery::matches`.
- `QueryData::fetch` now returns `QueryData::Item` directly, without an `Option`
  wrapper. Instead, implementations must provide `WorldQuery::matches` to
  decide whether or not a row matches the query. `QueryData::fetch` can use
  the return value of `matches` as a safety guarantee to avoid double-checking
  a condition, for example with `Option::unwrap_unchecked`.
- `WorldQuery` has a few new methods: `find_table_chunk` and `find_archetype_chunk`.
  These are unnecessary for most implementations, but add some optimization
  opportunities. See module docs for more info.
