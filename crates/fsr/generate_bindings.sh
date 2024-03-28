#!/bin/sh

bindgen fsr/include/wrapper.h \
    --allowlist-item ".*[fF][fF][xX].*" \
    --blocklist-item ".*[vV]k.*" \
    -o src/bindings.rs \
    --no-layout-tests \
    -- \
    -x c++ \
    -fdeclspec \
    -I $VULKAN_SDK/include
