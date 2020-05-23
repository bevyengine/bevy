use ron::value::Value;
use serde::Serialize;

fn main() {
    let data = r#"
        Scene( // class name is optional
            materials: { // this is a map
                "metal": (
                    reflectivity: 1.0,
                ),
                "plastic": (
                    reflectivity: 0.5,
                ),
            },
            entities: [ // this is an array
                (
                    name: "hero",
                    material: "metal",
                ),
                (
                    name: "monster",
                    material: "plastic",
                ),
            ],
        )
        "#;

    let value: Value = data.parse().expect("Failed to deserialize");
    let mut ser = serde_json::Serializer::pretty(std::io::stdout());
    value.serialize(&mut ser).expect("Failed to serialize");
}
