/// Lazily shortens a type name to remove all module paths.
///
/// The short name of a type is its full name as returned by
/// [`std::any::type_name`], but with the prefix of all paths removed. For
/// example, the short name of `alloc::vec::Vec<core::option::Option<u32>>`
/// would be `Vec<Option<u32>>`.
///
/// Shortening is performed lazily without allocation.
#[cfg_attr(
    feature = "alloc",
    doc = r#" To get a [`String`] from this type, use the [`to_string`](`alloc::string::ToString::to_string`) method."#
)]
///
/// # Examples
///
/// ```rust
/// # use bevy_reflect::ShortName;
/// #
/// # mod foo {
/// #     pub mod bar {
/// #         pub struct Baz;
/// #     }
/// # }
/// // Baz
/// let short_name = ShortName::of::<foo::bar::Baz>();
/// ```
#[derive(Clone, Copy)]
pub struct ShortName<'a>(pub &'a str);

impl ShortName<'static> {
    /// Gets a shortened version of the name of the type `T`.
    pub fn of<T: ?Sized>() -> Self {
        Self(core::any::type_name::<T>())
    }
}

impl<'a> ShortName<'a> {
    /// Gets the original name before shortening.
    pub const fn original(&self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for ShortName<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

impl<'a> core::fmt::Debug for ShortName<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let &ShortName(full_name) = self;
        // Generics result in nested paths within <..> blocks.
        // Consider "bevy_render::camera::camera::extract_cameras<bevy_render::camera::bundle::Camera3d>".
        // To tackle this, we parse the string from left to right, collapsing as we go.
        let mut index: usize = 0;
        let end_of_string = full_name.len();

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

                f.write_str(collapse_type_name(segment_to_collapse))?;

                // Insert the special character
                let special_character =
                    &rest_of_string[special_character_index..=special_character_index];

                f.write_str(special_character)?;

                match special_character {
                    ">" | ")" | "]"
                        if rest_of_string[special_character_index + 1..].starts_with("::") =>
                    {
                        f.write_str("::")?;
                        // Move the index past the "::"
                        index += special_character_index + 3;
                    }
                    // Move the index just past the special character
                    _ => index += special_character_index + 1,
                }
            } else {
                // If there are no special characters left, we're done!
                f.write_str(collapse_type_name(rest_of_string))?;
                index = end_of_string;
            }
        }

        Ok(())
    }
}

impl<'a> core::fmt::Display for ShortName<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as core::fmt::Debug>::fmt(self, f)
    }
}

#[inline(always)]
fn collapse_type_name(string: &str) -> &str {
    // Enums types are retained.
    // As heuristic, we assume the enum type to be uppercase.
    let mut segments = string.rsplit("::");
    let (last, second_last): (&str, Option<&str>) = (segments.next().unwrap(), segments.next());
    let Some(second_last) = second_last else {
        return last;
    };

    if second_last.starts_with(char::is_uppercase) {
        let index = string.len() - last.len() - second_last.len() - 2;
        &string[index..]
    } else {
        last
    }
}

#[cfg(all(test, feature = "alloc"))]
mod name_formatting_tests {
    use super::ShortName;

    #[test]
    fn trivial() {
        assert_eq!(ShortName("test_system").to_string(), "test_system");
    }

    #[test]
    fn path_separated() {
        assert_eq!(
            ShortName("bevy_prelude::make_fun_game").to_string(),
            "make_fun_game"
        );
    }

    #[test]
    fn tuple_type() {
        assert_eq!(
            ShortName("(String, String)").to_string(),
            "(String, String)"
        );
    }

    #[test]
    fn array_type() {
        assert_eq!(ShortName("[i32; 3]").to_string(), "[i32; 3]");
    }

    #[test]
    fn trivial_generics() {
        assert_eq!(ShortName("a<B>").to_string(), "a<B>");
    }

    #[test]
    fn multiple_type_parameters() {
        assert_eq!(ShortName("a<B, C>").to_string(), "a<B, C>");
    }

    #[test]
    fn enums() {
        assert_eq!(ShortName("Option::None").to_string(), "Option::None");
        assert_eq!(ShortName("Option::Some(2)").to_string(), "Option::Some(2)");
        assert_eq!(
            ShortName("bevy_render::RenderSet::Prepare").to_string(),
            "RenderSet::Prepare"
        );
    }

    #[test]
    fn generics() {
        assert_eq!(
            ShortName("bevy_render::camera::camera::extract_cameras<bevy_render::camera::bundle::Camera3d>").to_string(),
            "extract_cameras<Camera3d>"
        );
    }

    #[test]
    fn nested_generics() {
        assert_eq!(
            ShortName("bevy::mad_science::do_mad_science<mad_science::Test<mad_science::Tube>, bavy::TypeSystemAbuse>").to_string(),
            "do_mad_science<Test<Tube>, TypeSystemAbuse>"
        );
    }

    #[test]
    fn sub_path_after_closing_bracket() {
        assert_eq!(
            ShortName("bevy_asset::assets::Assets<bevy_scene::dynamic_scene::DynamicScene>::asset_event_system").to_string(),
            "Assets<DynamicScene>::asset_event_system"
        );
        assert_eq!(
            ShortName("(String, String)::default").to_string(),
            "(String, String)::default"
        );
        assert_eq!(
            ShortName("[i32; 16]::default").to_string(),
            "[i32; 16]::default"
        );
    }
}
