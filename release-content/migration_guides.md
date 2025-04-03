# Bevy's Migration Guide Process

Hi! Did someone add `M-Needs-Migration-Guide` to your PR? If so, you're in the right place.
Let's talk about how this process works.

When we make breaking changes to Bevy, we need to communicate them to users so their libraries and applications can be moved to the new Bevy version.
To do this, we write and ship a [migration guide](https://bevyengine.org/learn/migration-guides/introduction/) for every major Bevy version.
To avoid a crunch at the end of the cycle as we *write* all of these,
Bevy asks authors (and reviewers) to write a draft migration guide as part of the pull requests that make breaking changes.

## Where to put your migration guides

Each major Bevy version (e.g. 0.12, or 2.0) will get its own migration guide.
The draft migration guides for the current cycle are organized in the `bevyengine/bevy/release-content/migration-guides` folder.

When we publish our first release candidate for a cycle, these notes are merged together and moved from `bevyengine/bevy` into `bevyengine/bevy-website`,
where they will receive a final editing pass.

If your PR introduces a new breaking change relative to the previous version, you should start a new guide by copying [the template](./migration_guides_template.md) into a new file in the `migration-guides` folder.
You should also update the existing migration guides in the other files, if your change effects them.

## What to put in your draft migration guide

Migration guides are intended to briefly communicate:

- what has been changed since the last release?
- why did we make this breaking change?
- how can users migrate their existing code?

Draft migration guides *do not need to be polished*: it's okay if you're not a native English speaker or aren't a wordsmith.
Editing is easy; we just want to have an expert's view on the questions above.

When writing migration guides, prefer terse, technical language, and be sure to include terms that users might search for.
Migration guides are not read end-to-end: instead, they are navigated via Ctrl+F as the reader follows the compiler errors and bugs.

## Grouping changes into migration guides

Migration guides should reflect the complete experience of migrating from the last major Bevy version to the next one.
If there are *multiple* breaking changes layered on top of each other,
you should edit the existing migration guide, rather than write a new one.

While some brave users live on Bevy's `main` branch, we can trust them to use the draft migration guides and read the PRs in question if needed.

As a result, each draft migration should be given a clear name matching the section title.
These titles should reflect the name of the old feature that was broken or changed.

## Note on the `#[deprecated]` attribute

Rust provides a very helpful [`#[deprecated]` attribute](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute), which is a compiler-aware way to mark a piece of Rust code as obsolete and slated for removal.
This can be a nice a tool to ease migrations, because it downgrades errors to warnings and makes the migration information available right in the user's IDE.

However, it's not always possible to use this attribute, and Bevy does not consider it to be a substitute to a migration guide entry.
