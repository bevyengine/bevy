---
title: Accessible Label Component
authors: ["@viridia"]
pull_requests: [24308]
---

The `AccessibleLabel` component allows the a11y `label` property to be specified separately from
other a11y properties.

In most apps, the `label` property comes from application code rather than library code.
However, the design of `accesskit` requires that all a11y properties be stored in a single
large data structure contained in the `AccessibilityNode` component. This creates a usability
conflict with BSN and other methods of spawning complex hierarchies, where composing multiple
components is the primary means of behavioral reuse.

By putting the label in its own component, it can be used as a mixin within BSN templates, allowing
the label to be added by the widget user rather than the widget author.

Internally, this uses component hooks to sync the `AccessibilityNode` properties with the
payload of the `AccessibleLabel` component, satisfying the needs of `accesskit`.
