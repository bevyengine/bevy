# Release Checklist

## Pre-release

1. Check regressions tag.
2. Check appropriate milestone.
3. Check GitHub Projects page for staleness.
4. Update change log.
5. Create migration guide.
6. Write blog post.
7. Update book.
8. Bump version number for all crates, using the "Release" workflow.
9. Create tag on GitHub.
10. Bump `latest` tag to most recent release.

## Release

1. Release on crates.io
2. Announce on:
    1. HackerNews
    2. Twitter
    3. Reddit: /r/bevy, /r/rust, /r/rust_gamedev
    4. Discord: Bevy, Game Development in Rust, Rust Programming Language Community
    5. This Month in Rust Game Development newsletter
    6. This Week in Rust newsletter

## Post-release

1. Bump version number for all crates to next versions, as `0.X-dev`, using the "Post-release version bump" workflow, to ensure properly displayed version for [Dev Docs](https://dev-docs.bevyengine.org/bevy/index.html).
2. Update Bevy version used for Bevy book code validation to latest release.
