/// Shortens a type name to remove all module paths.
///
/// The short name of a type is its full name as returned by
/// [`std::any::type_name`], but with the prefix of all paths removed. For
/// example, the short name of `alloc::vec::Vec<core::option::Option<u32>>`
/// would be `Vec<Option<u32>>`.
pub fn get_short_name(full_name: &str) -> String {
    // Generics result in nested paths within <..> blocks.
    // Consider "bevy_render::camera::camera::extract_cameras<bevy_render::camera::bundle::Camera3d>".
    // To tackle this, we parse the string from left to right, collapsing as we go.
    let mut index: usize = 0;
    let end_of_string = full_name.len();
    let mut parsed_name = String::new();

    while index < end_of_string {
        let rest_of_string = full_name.get(index..end_of_string).unwrap_or_default();

        // Collapse everything up to the next special character,
        // then skip over it
        if let Some(special_character_index) = rest_of_string.find(|c: char| {
            (c == ' ')
                || (c == '<')
                || (c == '>')
                || (c == '(')
                || (c == ')')
                || (c == '[')
                || (c == ']')
                || (c == ',')
                || (c == ';')
        }) {
            let segment_to_collapse = rest_of_string
                .get(0..special_character_index)
                .unwrap_or_default();
            parsed_name += collapse_type_name(segment_to_collapse);
            // Insert the special character
            let special_character =
                &rest_of_string[special_character_index..=special_character_index];
            parsed_name.push_str(special_character);
            // Move the index just past the special character
            index += special_character_index + 1;
        } else {
            // If there are no special characters left, we're done!
            parsed_name += collapse_type_name(rest_of_string);
            index = end_of_string;
        }
    }
    parsed_name
}

#[inline(always)]
fn collapse_type_name(string: &str) -> &str {
    string.split("::").last().unwrap()
}

#[cfg(test)]
mod name_formatting_tests {
    use super::get_short_name;

    #[test]
    fn trivial() {
        assert_eq!(get_short_name("test_system"), "test_system");
    }

    #[test]
    fn path_seperated() {
        assert_eq!(
            get_short_name("bevy_prelude::make_fun_game"),
            "make_fun_game".to_string()
        );
    }

    #[test]
    fn tuple_type() {
        assert_eq!(
            get_short_name("(String, String)"),
            "(String, String)".to_string()
        );
    }

    #[test]
    fn array_type() {
        assert_eq!(get_short_name("[i32; 3]"), "[i32; 3]".to_string());
    }

    #[test]
    fn trivial_generics() {
        assert_eq!(get_short_name("a<B>"), "a<B>".to_string());
    }

    #[test]
    fn multiple_type_parameters() {
        assert_eq!(get_short_name("a<B, C>"), "a<B, C>".to_string());
    }

    #[test]
    fn generics() {
        assert_eq!(
            get_short_name("bevy_render::camera::camera::extract_cameras<bevy_render::camera::bundle::Camera3d>"),
            "extract_cameras<Camera3d>".to_string()
        );
    }

    #[test]
    fn nested_generics() {
        assert_eq!(
            get_short_name("bevy::mad_science::do_mad_science<mad_science::Test<mad_science::Tube>, bavy::TypeSystemAbuse>"),
            "do_mad_science<Test<Tube>, TypeSystemAbuse>".to_string()
        );
    }
}
