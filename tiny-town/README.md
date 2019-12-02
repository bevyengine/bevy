

```bash
# run using one of these commands
# lld linker makes compiles faster
# rust backtrace gives you a nice backtrace on panics
env RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo run --release
env RUSTFLAGS="-C link-arg=-fuse-ld=lld" RUST_BACKTRACE=1 cargo run --release
```