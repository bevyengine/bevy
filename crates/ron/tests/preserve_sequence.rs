use ron::{
    de::from_str,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    boolean: bool,
    float: f32,
    map: BTreeMap<u8, char>,
    nested: Nested,
    tuple: (u32, u32),
}

#[derive(Debug, Deserialize, Serialize)]
struct Nested {
    a: String,
    b: char,
}

fn read_original(source: &str) -> String {
    source.to_string().replace("\r\n", "\n")
}

fn make_roundtrip(source: &str) -> String {
    let config: Config = match from_str(source) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };
    let pretty = PrettyConfig::new()
        .with_depth_limit(3)
        .with_separate_tuple_members(true)
        .with_enumerate_arrays(true)
        .with_new_line("\n".into());
    to_string_pretty(&config, pretty).expect("Serialization failed")
}

#[test]
fn test_sequence_ex1() {
    let file = include_str!("preserve_sequence_ex1.ron");
    assert_eq!(read_original(file), make_roundtrip(file));
}

#[test]
fn test_sequence_ex2() {
    let file = include_str!("preserve_sequence_ex2.ron");
    assert_eq!(read_original(file), make_roundtrip(file));
}
