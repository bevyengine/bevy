export EXAMPLE=$1
export TS=$(date -u --iso-8601=seconds)
RELEASE=${2:-release}
if [ $RELEASE == "release" ]; then
   RELEASE_OPTS="--release"
fi
OUT_DIR=bevy-showcase
mkdir -p ${OUT_DIR}/wasm
cp -a assets/ ${OUT_DIR}
cargo build  $RELEASE_OPTS --example $EXAMPLE --target wasm32-unknown-unknown --no-default-features --features web;
wasm-bindgen --no-typescript --out-dir ${OUT_DIR}/wasm --out-name $EXAMPLE --target web ~/target/wasm32-unknown-unknown/$RELEASE/examples/${EXAMPLE}.wasm;
sed 's|\.js\$/, |\.js/, |' -i ${OUT_DIR}/wasm/${EXAMPLE}.js
envsubst < template_index.html > $OUT_DIR/${EXAMPLE}.html
echo
echo http://127.0.0.1:4000/${EXAMPLE}.html
#echo
test -z "$SKIP_HTTP_SERVER" && basic-http-server -x $OUT_DIR
