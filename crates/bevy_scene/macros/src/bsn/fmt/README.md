# BSN fmt

## Installation

```sh
cargo install --path ./crates/bevy_scene/macros --features fmt --bin bsnfmt
```

## Usage

Check:
```sh
bsnfmt ./path/to/module.rs
```

Write:
```sh
bsnfmt ./path/to/module.rs --write
```

## Editor integration

### nvim

#### conform

```lua
formatters_by_ft = {
    rust = { "rustfmt", "bsnfmt" },
},
formatters = {
    bsn_fmt = {
        command = "bsnfmt",
        stdin = true,
    },
},
```

