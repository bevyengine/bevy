---
title: Access::reads_and_writes has been renamed to reads.
pull_requests: [24778]
---

Previously `Access` contained (at least) `reads_and_writes` and `writes`. This was a confusing,
because it suggested that the former also implied write access. We've now renamed `reads_and_writes`
to just `reads` (since write access implies read access).

As such, the following members have been renamed:

- `Access::try_reads_and_writes` -> `Access::try_reads`
- `UnboundedAccessError::read_and_writes_inverted` -> `UnboundedAccessError::reads_inverted`
