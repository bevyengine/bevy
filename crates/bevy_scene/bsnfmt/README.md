# BSN fmt

## Installation

```sh
cargo install --path ./crates/bevy_scene/bsnfmt
```

## Usage

Check:

```sh
bsnfmt ./path/to/module.rs
```

Write:

```sh
bsnfmt --write ./path/to/module.rs
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
