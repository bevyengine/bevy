use ron::de::from_str;

#[test]
fn test_inf_and_nan() {
    assert_eq!(from_str("inf"), Ok(std::f64::INFINITY));
    assert_eq!(from_str("-inf"), Ok(std::f64::NEG_INFINITY));
    assert_eq!(from_str::<f64>("NaN").map(|n| n.is_nan()), Ok(true))
}
