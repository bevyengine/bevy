#!/usr/bin/env bash

# based on https://github.com/mozilla/glean/blob/main/build-scripts/xc-universal-binary.sh

set -eux

PATH=$PATH:$HOME/.cargo/bin

RELFLAG=
if [[ "$CONFIGURATION" != "Debug" ]]; then
    RELFLAG=--release
fi

set -euvx

# add path to the system SDK, needed since macOS 11
if [ -z ${LIBRARY_PATH+x} ]; then
    export LIBRARY_PATH="$(xcrun --show-sdk-path)/usr/lib"
else
    export LIBRARY_PATH="$LIBRARY_PATH:$(xcrun --show-sdk-path)/usr/lib"
fi

# add path to cmake, needed on apple arm processors as it's not available by default
if ! cmake --version; then
    # use the one installed from homebrew
    if /opt/homebrew/bin/cmake --version; then
        export PATH="$PATH:/opt/homebrew/bin"
    fi
fi

IS_SIMULATOR=0
if [ "${LLVM_TARGET_TRIPLE_SUFFIX-}" = "-simulator" ]; then
  IS_SIMULATOR=1
fi

for arch in $ARCHS; do
  case "$arch" in
    x86_64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        echo "Building for x86_64, but not a simulator build. What's going on?" >&2
        exit 2
      fi

      # Intel iOS simulator
      export CFLAGS_x86_64_apple_ios="-target x86_64-apple-ios"
      cargo build --lib $RELFLAG --target x86_64-apple-ios
      ;;

    arm64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        # Hardware iOS targets
        cargo build --lib $RELFLAG --target aarch64-apple-ios
      else
        # M1 iOS simulator -- currently in Nightly only and requires to build `libstd`
        cargo build --lib $RELFLAG --target aarch64-apple-ios-sim
      fi
  esac
done
