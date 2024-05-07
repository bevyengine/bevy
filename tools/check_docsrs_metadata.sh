#!/bin/bash

# Switch to the root project directory, letting this script be called from any folder.
cd $(dirname $0)/../

# Here we construct what a valid docs.rs metadata would look like. If you want to modify the
# required configuration, change these lines.
echo Constructing metadata requirements...

# Build crate with all features enabled and visible.
ALL_FEATURES='true'

# Scrape examples and embed them in documentation.
#   - https://doc.rust-lang.org/cargo/reference/unstable.html#scrape-examples
CARGO_ARGS='[ "-Zunstable-options", "-Zrustdoc-scrape-examples" ]'

# Enable `#[cfg(docsrs)]` blocks for all crates and dependencies, so they can use nightly features.
RUSTDOC_ARGS='[ "-Zunstable-options", "--cfg", "docsrs" ]'

# Combine all the options together into a single JSON string.
DOCSRS_METADATA="{ \"all-features\": $ALL_FEATURES, \"cargo-args\": $CARGO_ARGS, \"rustdoc-args\": $RUSTDOC_ARGS }"

echo Crates will be validated to ensure they have the following fields for \`[package.metadata.docs.rs]\`:
echo $DOCSRS_METADATA | jq --color-output '.' # Echo metadata using `jq`'s pretty-printing.

echo Gathering crate metadata for current workspace...

# Retrieve the metadata of all crates within this workspace.
CRATE_METADATA=$(cargo metadata --format-version 1 --no-deps)

# This filters the list of crates to only include crates that will be published to crates.io. It
# does this by removing all crates that set `publish = false`, or as `cargo-metadata` likes to say,
# `publish = []`. It then only selects the fields that we need: name, path, and docs.rs metadata.
PUBLIC_CRATES=$(
    echo $CRATE_METADATA | \
        jq '[.packages[] | select(.publish != []) | { name, docsrs: .metadata.docs.rs, manifest_path }]'
)

echo Validating all crates...

INVALID_CRATES=$(echo $PUBLIC_CRATES | jq "[.[] | select(.docsrs != $DOCSRS_METADATA)]")
# INVALID_CRATES=$(echo $PUBLIC_CRATES | jq "[.[] | select(.name == \"yef\")]")

PUBLIC_CRATES_LEN=$(echo $PUBLIC_CRATES | jq 'length')

# If there are no invalid crates.
if [[ $INVALID_CRATES == "[]" ]]; then
    echo All crates are valid! \(Checked $PUBLIC_CRATES_LEN.\)
    exit 0
else
    INVALID_CRATES_LEN=$(echo $INVALID_CRATES | jq 'length')

    # Log how many crates are invalid.
    echo $INVALID_CRATES_LEN crates are invalid \(out of $PUBLIC_CRATES_LEN\):

    # Iterate over each crate, splitting them up.
    for CRATE in $(echo $INVALID_CRATES | jq -c '.[]'); do
        # Format crate info to be "CRATE_NAME: CRATE_PATH"
        CRATE_NAME_PATH=$(echo $CRATE | jq '[.name, .manifest_path] | join(": ")')

        # Print crate info with fun colors! (ANSI escape codes)
        echo -e "- \033[31m$CRATE_NAME_PATH\033[0m" 
    done

    # Return with a non-zero exit code, denoting an error.
    exit 1
fi
