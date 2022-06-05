use xshell::{cmd, Shell};

fn main() {
    let example = std::env::args().nth(1).expect("abbb");
    let sh = Shell::new().unwrap();
    cmd!(
        sh,
        "cargo build --release --target wasm32-unknown-unknown --example {example}"
    )
    .run()
    .expect("Error building example");
    cmd!(
        sh,
        "wasm-bindgen --out-dir examples/wasm/target --out-name wasm_example --target web target/wasm32-unknown-unknown/release/examples/{example}.wasm"
    )
    .run()
    .expect("Error creating wasm binding");
}
