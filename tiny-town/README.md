

```bash
# run using one of these commands
# lld linker makes compiles faster
# rust backtrace gives you a nice backtrace on panics
# -Zshare-generics=y makes generics slightly faster for some reason
env RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo run --release
env RUSTFLAGS="-C link-arg=-fuse-ld=lld -Zshare-generics=y" RUST_BACKTRACE=1 cargo run --release
```