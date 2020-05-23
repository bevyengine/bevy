use ron::de::from_str;

#[test]
fn test_char() {
    let de: char = from_str("'Փ'").unwrap();
    assert_eq!(de, 'Փ');
}

#[test]
fn test_string() {
    let de: String = from_str("\"My string: ऄ\"").unwrap();
    assert_eq!(de, "My string: ऄ");
}
