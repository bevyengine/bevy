#!/bin/sh

set -e

PATH=$PATH:$HOME/.cargo/bin

# If you want your build to run faster, add a "--targets x86_64-apple-ios" for just using the ios simulator.
if [ -n ${IOS_TARGETS} ]; then
    cargo lipo --targets ${IOS_TARGETS}
else
    cargo lipo
fi
