#!/bin/sh
set -euxo pipefail

example_name="$1"

wasm_example_dir=$(dirname -- "$(readlink -f -- "${BASH_SOURCE[0]}")")
workspace_root=$(realpath "${wasm_example_dir}/../..")

cd "${workspace_root}"

example_output_dir="$wasm_example_dir/target/$example_name"
rm -rf "$example_output_dir"
mkdir -p "$example_output_dir"
rsync -a --exclude='target' --exclude='run-example.sh' "$wasm_example_dir/" "$example_output_dir/"

RUSTFLAGS='--cfg=web_sys_unstable_apis --cfg=getrandom_backend="wasm_js"' cargo build --release --example "$example_name" --target wasm32-unknown-unknown
wasm-bindgen --out-name wasm_example --out-dir "$example_output_dir/target" --target web "target/wasm32-unknown-unknown/release/examples/$example_name.wasm"

basic-http-server "$example_output_dir"
