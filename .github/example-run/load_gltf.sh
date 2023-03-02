#!/bin/sh
exec cargo run --example load_gltf --features "bevy_ci_testing,trace,trace_chrome,debug_asset_server"