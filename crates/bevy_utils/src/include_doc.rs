/// Allows for compile-time deduplication of documentation for commonly repeated documentation.
/// Assumes that the crate is compiling under bevy-root-dir/crates/some-crate-name/ ...
///
/// ```rust
/// use bevy_utils::include_doc;
///
/// struct MyType {
///   /// Inherited visibility of an entity.
///   inherited_visibility_duplicated: i32,
///
///   #[doc = include_doc!(inherited_visibility)]
///   inherited_visibility_better_docs: u32,
/// }
/// ```
#[macro_export]
macro_rules! include_doc {
    (inherited_visibility) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../inline-docs/inherited_visibility.md"
        ))
    };
		(visibility) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../inline-docs/visibility.md"
        ))
    };
		(view_visibility) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../inline-docs/view_visibility.md"
        ))
    };
}
