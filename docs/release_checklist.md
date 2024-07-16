# Release Checklist

## Minor Version

### Minor Pre-release

1. Check regressions tag.
2. Check appropriate milestone, and close it.
3. Check GitHub Projects page for staleness.
4. Update change log.
5. Create migration guide.
6. Write blog post.
7. Update book.
8. Bump version number for all crates, using the "Release" workflow.
    * Change the commit message to be nicer
9. Create tag on GitHub.
10. Edit Github Release. Add links to the `Release announcement` and `Migration Guide`.
11. Bump `latest` tag to most recent release.
12. Run this workflow to update screenshots:
    * <https://github.com/bevyengine/bevy-website/actions/workflows/update-screenshots.yml>
    * _This will block blog post releases (and take ~40 minutes) so do it early_.
13. Run this workflow to update wasm examples:
    * <https://github.com/bevyengine/bevy-website/actions/workflows/build-wasm-examples.yml>

### Minor Release

1. Release on crates.io
    * `bash tools/publish.sh`
2. Announce on:
    1. HackerNews
    2. Twitter
    3. Reddit: /r/bevy, /r/rust, /r/rust_gamedev
    4. Discord: Bevy, Game Development in Rust, Rust Programming Language Community
    5. This Month in Rust Game Development newsletter
    6. This Week in Rust newsletter

### Minor Post-release

1. Bump version number for all crates to next versions, as `0.X-dev`, using the "Post-release version bump" workflow, to ensure properly displayed version for [Dev Docs](https://dev-docs.bevyengine.org/bevy/index.html).
2. Update Bevy version used for Bevy book code validation to latest release.

## Patch

### Patch Pre-release

1. Check appropriate milestone.
2. Close the milestone, open the next one if anything remains and transfer them.
3. Bump version number for all crates, using the command from the "Release" workflow locally, with `patch` for the new version. At the time of writing this:
    * `cargo release patch --workspace --no-publish --execute --no-tag --no-confirm --no-push --dependent-version upgrade --exclude ci --exclude errors --exclude bevy_mobile_example --exclude build-wasm-example`
    * Change the commit message to be nicer
4. Create tag on GitHub.
5. Edit Github Release. Add link to the comparison between this patch and the previous version.
6. Bump `latest` tag to most recent release.
7. Run this workflow to update screenshots:
    * <https://github.com/bevyengine/bevy-website/actions/workflows/update-screenshots.yml>
8. Run this workflow to update wasm examples:
    * <https://github.com/bevyengine/bevy-website/actions/workflows/build-wasm-examples.yml>

### Patch Release

1. Release on crates.io
    * `bash tools/publish.sh`
2. Announce on:
    1. Discord: Bevy

### Patch Post-Release

## Release Candidate

### RC Pre-Release

1. Check appropriate milestone.
2. Create a branch for the release.
3. Bump version number for all crates, using the command from the "Release" workflow locally, with `rc` for the new version. At the time of writing this:
    * `cargo release rc --workspace --no-publish --execute --no-tag --no-confirm --no-push --dependent-version upgrade --exclude ci --exclude errors --exclude bevy_mobile_example --exclude build-wasm-example`
    * Change the commit message to be nicer
4. Create tag on GitHub.
5. Edit Github Release. Add link to the comparison between this rc and the previous version.

### RC Release

1. Release on crates.io
    * `bash tools/publish.sh`
2. Announce on:
    1. Discord: Bevy, #dev-announcements

### RC Post-Release

1. Update Bevy version used for Bevy book code validation to latest release.
