use syn::Path;

/// The type of a [`Path`], using standard Rust naming conventions to infer the type.
/// Note that this only works for paths that actually follow the naming conventions, and there are
/// inherent ambiguities, such as `Some` either referring to a type or an enum variant.
#[derive(PartialEq, Eq, Debug)]
pub enum PathType {
    /// Path that follows Rust type conventions.
    Type,
    /// Path that follows Rust enum conventions.
    Enum,
    /// Path that follows Rust const conventions.
    Const,
    /// Path that follows Rust type-associated const conventions.
    TypeConst,
    /// Path that follows Rust type-associated function conventions.
    TypeFunction,
    /// Path that follows Rust function conventions.
    Function,
}

impl PathType {
    /// Determines the [`PathType`] for the given path.
    pub fn new(path: &Path) -> PathType {
        let mut iter = path.segments.iter().rev();
        if let Some(last_segment) = iter.next() {
            let last_string = last_segment.ident.to_string();
            let mut last_string_chars = last_string.chars();
            let last_ident_first_char = last_string_chars.next().unwrap();
            if last_ident_first_char.is_uppercase() {
                let is_const = is_const(&last_string);
                if let Some(second_to_last_segment) = iter.next() {
                    // PERF: is there some way to avoid this string allocation?
                    let second_to_last_string = second_to_last_segment.ident.to_string();
                    let first_char = second_to_last_string.chars().next().unwrap();
                    if first_char.is_uppercase() {
                        if is_const {
                            PathType::TypeConst
                        } else {
                            PathType::Enum
                        }
                    } else if is_const {
                        PathType::Const
                    } else {
                        PathType::Type
                    }
                } else if is_const {
                    PathType::Const
                } else {
                    PathType::Type
                }
            } else if let Some(second_to_last) = iter.next() {
                // PERF: is there some way to avoid this string allocation?
                let second_to_last_string = second_to_last.ident.to_string();
                let first_char = second_to_last_string.chars().next().unwrap();
                if first_char.is_uppercase() {
                    PathType::TypeFunction
                } else {
                    PathType::Function
                }
            } else {
                PathType::Function
            }
        } else {
            // This won't be hit so just pick one to make it easy on consumers
            PathType::Type
        }
    }
}

fn is_const(path: &str) -> bool {
    // Paths of length 1 are ambiguous, we give the tie to Types,
    // as that is more useful for scenes
    if path.len() == 1 {
        return false;
    }

    // All characters are uppercase ... this is a Const
    !path.chars().any(char::is_lowercase)
}

#[cfg(test)]
mod tests {
    use super::{is_const, PathType};
    use syn::{parse_str, Path};

    macro_rules! test_path_type {
        ($test_name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                // Arrange
                let path = parse_str::<Path>($input).unwrap();
                let expected = $expected;

                // Act
                let result = PathType::new(&path);

                // Assert
                assert_eq!(result, expected, "Failed on path: '{}'", $input);
            }
        };
    }

    // Types
    test_path_type!(path_type_standard_root, "XType", PathType::Type);
    test_path_type!(path_type_standard_namespace, "foo::XType", PathType::Type);

    // These cases are ambiguous. We parse it as a Type as that works better in the scene patching context.
    test_path_type!(path_type_ambiguous_single_char_root, "X", PathType::Type);
    test_path_type!(
        path_type_ambiguous_single_char_namespace,
        "foo::X",
        PathType::Type
    );

    // Constants
    test_path_type!(path_type_const_root, "X_AXIS", PathType::Const);
    test_path_type!(path_type_const_namespace, "foo::X_AXIS", PathType::Const);
    test_path_type!(path_type_const_no_underscore_root, "XAXIS", PathType::Const);
    test_path_type!(
        path_type_const_no_underscore_namespace,
        "foo::XAXIS",
        PathType::Const
    );

    // Enums
    test_path_type!(path_type_enum_standard, "Foo::Bar", PathType::Enum);
    test_path_type!(path_type_enum_namespace, "foo::Foo::Bar", PathType::Enum);

    // This is ambiguous with TypeConst ... we give the tie to Enum as that works better in a scene context.
    test_path_type!(
        path_type_enum_ambiguous_single_char,
        "Foo::B",
        PathType::Enum
    );

    // Type Functions
    test_path_type!(
        path_type_type_function_standard,
        "Foo::bar",
        PathType::TypeFunction
    );
    test_path_type!(
        path_type_type_function_namespace,
        "foo::Foo::bar",
        PathType::TypeFunction
    );

    // Type Constants
    test_path_type!(
        path_type_type_const_standard,
        "Foo::BAR",
        PathType::TypeConst
    );

    // Functions
    test_path_type!(path_type_function_root, "foo", PathType::Function);
    test_path_type!(path_type_function_namespace, "foo::foo", PathType::Function);
    test_path_type!(path_type_function_single_char, "f", PathType::Function);

    macro_rules! test_is_const {
        ($test_name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                // Arrange
                let input = $input;
                let expected = $expected;

                // Act
                let result = is_const(input);

                // Assert
                assert_eq!(result, expected, "Failed on input: '{}'", input);
            }
        };
    }

    // Length == 1
    test_is_const!(single_upper_is_not_const, "X", false);
    test_is_const!(single_lower_is_not_const, "a", false);

    // Valid
    test_is_const!(standard_const_with_underscore, "X_AXIS", true);
    test_is_const!(standard_const_max_value, "MAX_VALUE", true);
    test_is_const!(multiple_upper_no_underscore, "PI", true);

    // Mixed casing
    test_is_const!(mixed_case_with_underscore_fails, "FOO_bar", false);
    test_is_const!(short_mixed_case_with_underscore_fails, "A_b", false);

    // Types & Functions
    test_is_const!(pascal_case_is_not_const, "Transform", false);
    test_is_const!(snake_case_is_not_const, "my_function", false);
    test_is_const!(camel_case_is_not_const, "camelCase", false);
}
