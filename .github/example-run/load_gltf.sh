#!/bin/sh
exec cargo run --example load_gltf --features $CI_FEATURES --features "debug_asset_server"