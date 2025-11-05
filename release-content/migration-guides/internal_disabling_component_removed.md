---
title: "`Internal` has been removed"
pull_requests: [ 21623 ]
---

The `Internal` component, previously added as a required component to both one-shot systems and observer entities has been removed.

You can remove all references to it: these entities are no longer hidden by default query filters.
If you have tests which rely on a specific number of entities existing in the world,
you should refactor them to query for entities with a component that you care about:
this is much more robust in general.

This component was previously motivated by two factors:

1. A desire to protect users from accidentally modifying engine internals, breaking their app in subtle and complex ways.
2. A unified API for entity inspectors, allowing them to readily distinguish between "engine-internal" and "user-defined" entities.

In practice, we found that this increased user friction and confusion without meaningfully improving robustness.
Entity inspectors and similar tools can and should define their own entity categorization functionality:
simply lumping all "internal" entities together is rarely helpful.
