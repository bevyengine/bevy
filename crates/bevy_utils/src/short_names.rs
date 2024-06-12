use std::{borrow::Cow, fmt, str};

const SPECIAL_TYPE_CHARS: [u8; 9] = *b" <>()[],;";
/// Shortens a type name to remove all module paths.
///
/// The short name of a type is its full name as returned by
/// [`std::any::type_name`], but with the prefix of all paths removed. For
/// example, the short name of `alloc::vec::Vec<core::option::Option<u32>>`
/// would be `Vec<Option<u32>>`.
pub fn get_short_name(full_name: &str) -> Cow<str> {
    // Generics result in nested paths within <..> blocks.
    // Consider "bevy_render::camera::camera::extract_cameras<bevy_render::camera::bundle::Camera3d>".
    // To tackle this, we parse the string from left to right, collapsing as we go.
    let mut remaining = full_name.as_bytes();
    let mut parsed_name = Vec::new();
    let mut complex_type = false;

    loop {
        // Collapse everything up to the next special character,
        // then skip over it
        let is_special = |c| SPECIAL_TYPE_CHARS.contains(c);
        if let Some(next_special_index) = remaining.iter().position(is_special) {
            complex_type = true;
            if parsed_name.is_empty() {
                parsed_name.reserve(remaining.len());
            }
            let (pre_special, post_special) = remaining.split_at(next_special_index + 1);
            parsed_name.extend_from_slice(collapse_type_name(pre_special));
            match pre_special.last().unwrap() {
                b'>' | b')' | b']' if post_special.get(..2) == Some(b"::") => {
                    parsed_name.extend_from_slice(b"::");
                    // Move the index past the "::"
                    remaining = &post_special[2..];
                }
                // Move the index just past the special character
                _ => remaining = post_special,
            }
        } else if !complex_type {
            let collapsed = collapse_type_name(remaining);
            // SAFETY: We only split on ASCII characters, and the input is valid UTF8, since
            // it was a &str
            let str = unsafe { str::from_utf8_unchecked(collapsed) };
            return Cow::Borrowed(str);
        } else {
            // If there are no special characters left, we're done!
            parsed_name.extend_from_slice(collapse_type_name(remaining));
            // SAFETY: see above
            let utf8_name = unsafe { String::from_utf8_unchecked(parsed_name) };
            return Cow::Owned(utf8_name);
        }
    }
}

/// Wrapper around `AsRef<str>` that uses the [`get_short_name`] format when
/// displayed.
pub struct DisplayShortName<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> fmt::Display for DisplayShortName<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let as_short_name = get_short_name(self.0.as_ref());
        write!(f, "{as_short_name}")
    }
}

#[inline(always)]
fn collapse_type_name(string: &[u8]) -> &[u8] {
    let find = |(index, window)| (window == b"::").then_some(index + 2);
    let split_index = string.windows(2).enumerate().rev().find_map(find);
    &string[split_index.unwrap_or(0)..]
}

#[cfg(test)]
mod name_formatting_tests {
    use super::get_short_name;

    #[test]
    fn trivial() {
        assert_eq!(get_short_name("test_system"), "test_system");
    }

    #[test]
    fn path_separated() {
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
    fn enums() {
        assert_eq!(get_short_name("Option::None"), "Option::None".to_string());
        assert_eq!(
            get_short_name("Option::Some(2)"),
            "Option::Some(2)".to_string()
        );
        assert_eq!(
            get_short_name("bevy_render::RenderSet::Prepare"),
            "RenderSet::Prepare".to_string()
        );
    }

    #[test]
    fn generics() {
        assert_eq!(
            get_short_name("bevy_render::camera::camera::extract_cameras<bevy_render::camera::bundle::Camera3d>"),
            "extract_cameras<Camera3d>".to_string()
        );
    }

    #[test]
    fn utf8_generics() {
        assert_eq!(
            get_short_name("bévï::camérą::łørđ::_öñîòñ<ķràźÿ::Москва::東京>"),
            "_öñîòñ<東京>".to_string()
        );
    }

    #[test]
    fn nested_generics() {
        assert_eq!(
            get_short_name("bevy::mad_science::do_mad_science<mad_science::Test<mad_science::Tube>, bavy::TypeSystemAbuse>"),
            "do_mad_science<Test<Tube>, TypeSystemAbuse>".to_string()
        );
    }

    #[test]
    fn sub_path_after_closing_bracket() {
        assert_eq!(
            get_short_name("bevy_asset::assets::Assets<bevy_scene::dynamic_scene::DynamicScene>::asset_event_system"),
            "Assets<DynamicScene>::asset_event_system".to_string()
        );
        assert_eq!(
            get_short_name("(String, String)::default"),
            "(String, String)::default".to_string()
        );
        assert_eq!(
            get_short_name("[i32; 16]::default"),
            "[i32; 16]::default".to_string()
        );
    }
}
