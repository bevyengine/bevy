---
title: The `validate_parent_has_component` is superseded by `ValidateParentHasComponentPlugin`
pull_requests: [22675]
---

The `validate_parent_has_component` insert hook has been replaced by a plugin:
`ValidateParentHasComponentPlugin`. This uses an observer, a resource, and a system to achieve a
more robust (and less spurious) warning for invalid configuration of entities.
