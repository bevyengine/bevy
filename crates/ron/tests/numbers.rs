use ron::de::from_str;

#[test]
fn test_hex() {
    assert_eq!(from_str("0x507"), Ok(0x507));
    assert_eq!(from_str("0x1A5"), Ok(0x1A5));
    assert_eq!(from_str("0x53C537"), Ok(0x53C537));
}

#[test]
fn test_bin() {
    assert_eq!(from_str("0b101"), Ok(0b101));
    assert_eq!(from_str("0b001"), Ok(0b001));
    assert_eq!(from_str("0b100100"), Ok(0b100100));
}

#[test]
fn test_oct() {
    assert_eq!(from_str("0o1461"), Ok(0o1461));
    assert_eq!(from_str("0o051"), Ok(0o051));
    assert_eq!(from_str("0o150700"), Ok(0o150700));
}
