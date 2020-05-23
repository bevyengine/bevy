use ron::value::Number;

#[test]
fn test_large_number() {
    use ron::value::Value;
    let test_var = Value::Number(Number::new(10000000000000000000000.0f64));
    let test_ser = ron::ser::to_string(&test_var).unwrap();
    let test_deser = ron::de::from_str::<Value>(&test_ser);

    assert_eq!(
        test_deser.unwrap(),
        Value::Number(Number::new(10000000000000000000000.0))
    );
}
