#[cfg(feature = "indexmap")]
use ron::{de::from_str, Value};

#[test]
#[cfg(feature = "indexmap")]
fn test_order_preserved() {
    let file = r#"(
tasks: {
    "debug message": Dbg(
      msg: "test message. some text after it."
    ),
    "shell command": Shell(
      command: "ls",
      args: Some([
        "-l",
        "-h",
      ]),
      ch_dir: Some("/"),
    ),
},
)
"#;

    let value: Value = from_str(file).unwrap();
    match value {
        Value::Map(map) => match &map[&Value::String("tasks".to_owned())] {
            Value::Map(map) => {
                assert_eq!(
                    *map.keys().next().unwrap(),
                    Value::String("debug message".to_string())
                );
                assert_eq!(
                    *map.keys().skip(1).next().unwrap(),
                    Value::String("shell command".to_string())
                );
            }
            _ => panic!(),
        },
        _ => panic!(),
    }

    let file = r#"(
tasks: {
    "shell command": Shell(
      command: "ls",
      args: Some([
        "-l",
        "-h",
      ]),
      ch_dir: Some("/")
    ),
    "debug message": Dbg(
      msg: "test message. some text after it."
    ),
}
)
"#;

    let value: Value = from_str(file).unwrap();
    match value {
        Value::Map(map) => match &map[&Value::String("tasks".to_owned())] {
            Value::Map(map) => {
                assert_eq!(
                    *map.keys().next().unwrap(),
                    Value::String("shell command".to_string())
                );
                assert_eq!(
                    *map.keys().skip(1).next().unwrap(),
                    Value::String("debug message".to_string())
                );
            }
            _ => panic!(),
        },
        _ => panic!(),
    }
}
