---
title: "Extra bounds on Specializer and SpecializerKey"
pull_requests: [20391]
---

- `Specializer` gained a `'static` bound
- `SpecializerKey` gained the following bounds: `Send + Sync + 'static`
- `SpecializerKey::Canonical` is now bounded by `SpecializerKey`
  rather than `Hash + Eq`
