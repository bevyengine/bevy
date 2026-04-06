# BSN fmt

```sh
cargo install --path .\crates\bevy_scene2\macros --features fmt --bin bsnfmt
```

```sh
bsnfmt .\examples\scene\bsn.rs
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

