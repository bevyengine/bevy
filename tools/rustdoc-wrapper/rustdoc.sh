#!/bin/env sh

# Gets clobbered, so make a backup.
export SET_CARGO_MANIFEST_PATH="$CARGO_MANIFEST_PATH"
export SET_CARGO_MANIFEST_DIR="$CARGO_MANIFEST_DIR"
# Pass on all arguments to our rustdoc wrapper.
# Use a different target directory because the workspace-level one
# will be locked.
cargo run --package rustdoc-wrapper --target-dir tools/rustdoc-wrapper/target --color always -- "$@"
