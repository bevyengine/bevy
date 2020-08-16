# Debugging

## Macro Debugging

* Print the final output of a macro using `cargo rustc --profile=check -- -Zunstable-options --pretty=expanded`
  * Alternatively you could install and use [cargo expand](https://github.com/dtolnay/cargo-expand) which adds syntax highlighting to the terminal output.
    * Additionally get pager by piping to `less` ( on Unix systems ): `cargo expand --color always | less -R`
* Print output during macro compilation using `eprintln!("hi");`

## WGPU Tracing

When a suspected wgpu error occurs, you should capture a wgpu trace so that Bevy and wgpu devs can debug using the [wgpu player tool](https://github.com/gfx-rs/wgpu/wiki/Debugging-wgpu-Applications#tracing-infrastructure).

To capture a wgpu trace:
1. create a new `wgpu_trace` folder in the root of your cargo workspace
2. add the "wgpu_trace" feature to the bevy crate. (ex: `cargo run --example features wgpu_trace`)
3. zip up the wgpu_trace folder and attach it to the relevant issue
