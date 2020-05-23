use ron::value::{Map, Number, Value};
use serde::Serialize;

#[test]
fn bool() {
    assert_eq!("true".parse(), Ok(Value::Bool(true)));
    assert_eq!("false".parse(), Ok(Value::Bool(false)));
}

#[test]
fn char() {
    assert_eq!("'a'".parse(), Ok(Value::Char('a')));
}

#[test]
fn map() {
    let mut map = Map::new();
    map.insert(Value::Char('a'), Value::Number(Number::new(1)));
    map.insert(Value::Char('b'), Value::Number(Number::new(2f64)));
    assert_eq!("{ 'a': 1, 'b': 2.0 }".parse(), Ok(Value::Map(map)));
}

#[test]
fn number() {
    assert_eq!("42".parse(), Ok(Value::Number(Number::new(42))));
    assert_eq!("3.1415".parse(), Ok(Value::Number(Number::new(3.1415f64))));
}

#[test]
fn option() {
    let opt = Some(Box::new(Value::Char('c')));
    assert_eq!("Some('c')".parse(), Ok(Value::Option(opt)));
}

#[test]
fn string() {
    let normal = "\"String\"";
    assert_eq!(normal.parse(), Ok(Value::String("String".into())));

    let raw = "r\"Raw String\"";
    assert_eq!(raw.parse(), Ok(Value::String("Raw String".into())));

    let raw_hashes = "r#\"Raw String\"#";
    assert_eq!(raw_hashes.parse(), Ok(Value::String("Raw String".into())));

    let raw_escaped = "r##\"Contains \"#\"##";
    assert_eq!(
        raw_escaped.parse(),
        Ok(Value::String("Contains \"#".into()))
    );

    let raw_multi_line = "r\"Multi\nLine\"";
    assert_eq!(
        raw_multi_line.parse(),
        Ok(Value::String("Multi\nLine".into()))
    );
}

#[test]
fn seq() {
    let seq = vec![
        Value::Number(Number::new(1)),
        Value::Number(Number::new(2f64)),
    ];
    assert_eq!("[1, 2.0]".parse(), Ok(Value::Seq(seq)));
}

#[test]
fn unit() {
    use ron::error::{Error, ErrorCode, Position};

    assert_eq!("()".parse(), Ok(Value::Unit));
    assert_eq!("Foo".parse(), Ok(Value::Unit));

    assert_eq!(
        "".parse::<Value>(),
        Err(Error {
            code: ErrorCode::Eof,
            position: Position { col: 1, line: 1 }
        })
    );
}

#[derive(Serialize)]
struct Scene(Option<(u32, u32)>);

#[derive(Serialize)]
struct Scene2 {
    foo: Option<(u32, u32)>,
}

#[test]
fn roundtrip() {
    use ron::{de::from_str, ser::to_string};

    {
        let s = to_string(&Scene2 {
            foo: Some((122, 13)),
        })
        .unwrap();
        println!("{}", s);
        let scene: Value = from_str(&s).unwrap();
        println!("{:?}", scene);
    }
    {
        let s = to_string(&Scene(Some((13, 122)))).unwrap();
        println!("{}", s);
        let scene: Value = from_str(&s).unwrap();
        println!("{:?}", scene);
    }
}
