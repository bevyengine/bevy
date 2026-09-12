# Build Tailwind Palette

Generates the Tailwind CSS color palette used by `bevy_color`.

## Updating the palette

1. Copy the contents of `hexColors` from [`color.tsx`](https://github.com/tailwindlabs/tailwindcss.com/blob/main/src/components/color.tsx).
2. Paste the copied contents into the `raw_js` variable in `read_colors()` in `main.rs`.
3. Run:
```sh
cargo run -p build-tailwind-palette
```

This updates:
```text
crates/bevy_color/src/palettes/tailwind.rs
```
