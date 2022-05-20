# Objective

Describe the objective or issue this PR addresses.

If you're fixing a specific issue, say "Fixes #X" and the linked issue will automatically be closed when this PR is merged.
Under each bullet point, describe how this change addressed those objectives if it is not obvious.

**Changes that will affect external library users must update RELEASES.md before they will be merged.**

## Context

Discuss any context that may be needed for a user with only passing acquaintance with this library to understand the changes you've made.
This may include related issues, previous discussion, or relevant bits of how the library works).

## Feedback wanted

> This section is optional. If there are no particularly tricky or controversial changes, you can delete this section.

Which parts of this PR were you unsure about? Which parts were particularly tricky?

If you're stuck on part of the changes or want feedback early, open a draft PR and list the items that need to be completed here using a checklist.

## Changelog

> This section is optional. If this was a trivial fix, or has no externally-visible impact, you can delete this section.

- What changed as a result of this PR?
- If applicable, organize changes under "Added", "Changed", or "Fixed" sub-headings
- Stick to one or two sentences. If more detail is needed for a particular change, consider adding it to the "Solution" section
  - If you can't summarize the work, your change may be unreasonably large / unrelated. Consider splitting your PR to make it easier to review and merge!

## Migration Guide

> This section is optional. If there are no breaking changes, you can delete this section.

- If this PR is a breaking change (relative to the last release of this library), describe how a user might need to migrate their code to support these changes
- Simply adding new functionality is not a breaking change.
- Fixing behavior that was definitely a bug, rather than a questionable design choice, is not a breaking change.
