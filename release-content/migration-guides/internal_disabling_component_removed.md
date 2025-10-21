---
title: "`Internal` has been removed"
pull_requests: [ TODO ]
---

The `Internal` component, previously added as a required component to both one-shot systems and observer entities has been removed.
You can remove all references to it: these entities are no longer hidden by default query filters.

This component was previously motivated by two factors:

1. A desire to protect users from accidentally modifying engine internals, breaking their app in subtle and complex ways.
2. A unified API for entity inspectors, allowing them to readily distinguish between "engine-internal" and "user-defined" entities.

In practice, we found that this increased user friction and confusion without meaningfully improving robustness.
Entity inspectors and similar tools can and should define their own entity categorization functionality:
simply lumping all "internal" entities together is rarely helpful.
