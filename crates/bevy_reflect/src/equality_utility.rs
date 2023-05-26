//! Utilities for performing partial equality checks on [`Reflect`] types.

use crate::{Reflect, TypeInfo};

/// Helper macro for extracting type information for use in [`Reflect::reflect_partial_eq`] implementations.
///
/// `$info` is an [`Option<TypeInfo>`] that must match a pattern, `$pat`, and
/// return expression, `$expr`, with type `Option<T>`.
///
/// If `$info` does not match the givn pattern or `None`, the macro performs an early return
/// with the value `Some(false)`.
///
/// [`Option<TypeInfo>`]: TypeInfo
macro_rules! extract_info {
    ($info:ident, $pat:pat => $expr:expr) => {{
        match $info {
            None => None,
            Some($pat) => $expr,
            _ => return Some(false),
        }
    }};
}

/// Helper macro for comparing two struct-like types with [`Reflect::reflect_partial_eq`].
///
/// By using a macro, we are able to share logic between standard structs and struct variants.
macro_rules! compare_structs {
    ($a:ident, $b:ident, $info_a:ident, $info_b:ident $(, accessor=.$field_accessor:ident)? $(,)?) => {{
        // By default, we only need to perform a separate check on `$b` if it has
        // a different number of fields than `$a`.
        let mut needs_field_check = $a.field_len() != $b.field_len();

        if $info_a.is_none() && $info_b.is_none() {
            // Both are pure dynamic types, so no fields are skippable
            if $a.field_len() != $b.field_len() {
                return Some(false);
            }
        }

        for (index, value_a) in $a.iter_fields().enumerate() {
            let value_a = value_a $(.$field_accessor())?;
            let field_name = $a.name_at(index)?;

            let field_a = $info_a.and_then(|info| info.field(field_name));
            let field_b = $info_b.and_then(|info| info.field(field_name));
            match (field_a, field_b) {
                (Some(field_a), Some(field_b)) => {
                    if field_a.meta().skip_partial_eq && field_b.meta().skip_partial_eq {
                        // Both fields have opted out of partial equality
                        continue;
                    }

                    if field_a.meta().skip_partial_eq != field_b.meta().skip_partial_eq {
                        // Only one of the fields is required for partial equality
                        return Some(false);
                    }
                }
                (Some(field_info), None) => {
                    if field_info.meta().skip_partial_eq {
                        // Field doesn't exist on `$b`, but is skipped by `$a`.
                        // If the number of fields are the same, we have to check
                        // if the corresponding field on `$b` is also skipped.
                        needs_field_check |= $a.field_len() == $b.field_len();
                        continue;
                    }
                }
                (None, Some(field_info)) => {
                    if field_info.meta().skip_partial_eq {
                        // Field exists on `$a` and `$b`, but is skipped by `$b`.
                        continue;
                    }
                }
                (None, None) => {}
            }

            let Some(value_b) = $b.field(field_name) else { return Some(false); };
            let is_equal = value_a.reflect_partial_eq(value_b)?;

            if !is_equal {
                return Some(false);
            }
        }

        // If 1 or more fields are missing from `b`, we need to check that they
        // are all marked as `skip_partial_eq` so as to preserve symmetry.
        if needs_field_check {
            for (index, _) in $b.iter_fields().enumerate() {
                let field_name = $b.name_at(index)?;
                if let Some(field_b) = $info_b.and_then(|info| info.field(field_name)) {
                    if field_b.meta().skip_partial_eq {
                        continue;
                    }
                }

                if $a.field(field_name).is_none() {
                    return Some(false);
                }
            }
        }

        Some(true)
    }};
}

/// Helper macro for comparing two tuple struct-like types with [`Reflect::reflect_partial_eq`].
///
/// By using a macro, we are able to share logic between standard tuple structs and tuple variants.
macro_rules! compare_tuple_structs {
    ($a:ident, $b:ident, $info_a:ident, $info_b:ident, accessor=.$field_accessor:ident $(,)?) => {{
        if $info_a.is_none() && $info_b.is_none() {
            // Both are pure dynamic types, so no fields are skippable
            if $a.field_len() != $b.field_len() {
                return Some(false);
            }
        }

        let a_len = $a.field_len();
        let b_len = $b.field_len();
        let max_len = a_len.max(b_len);

        for index in 0..max_len {
            let field_a = $info_a.and_then(|info| info.field_at(index));
            let field_b = $info_b.and_then(|info| info.field_at(index));
            match (field_a, field_b) {
                (Some(field_a), Some(field_b)) => {
                    if field_a.meta().skip_partial_eq && field_b.meta().skip_partial_eq {
                        // Both fields have opted out of partial equality
                        continue;
                    }

                    if field_a.meta().skip_partial_eq != field_b.meta().skip_partial_eq {
                        // Only one of the fields is required for partial equality
                        return Some(false);
                    }
                }
                (Some(field_info), None) | (None, Some(field_info)) => {
                    if field_info.meta().skip_partial_eq {
                        continue;
                    }
                }
                (None, None) => {}
            }

            let Some(value_a) = $a.$field_accessor(index) else { return Some(false); };
            let Some(value_b) = $b.$field_accessor(index) else { return Some(false); };
            let is_equal = value_a.reflect_partial_eq(value_b)?;

            if !is_equal {
                return Some(false);
            }
        }

        Some(true)
    }};
}

pub(crate) use compare_structs;
pub(crate) use compare_tuple_structs;
pub(crate) use extract_info;

/// Helper function for returning a tuple of [`Option<TypeInfo>'s`] for two [`Reflect`] values.
///
/// This function will try to be smart about when it calls [`Reflect::get_represented_type_info`]
/// so that it avoids calling the method twice for concrete values of the same type.
///
/// [`Option<TypeInfo>'s`]: TypeInfo
pub(crate) fn get_type_info_pair(
    a: &dyn Reflect,
    b: &dyn Reflect,
) -> (Option<&'static TypeInfo>, Option<&'static TypeInfo>) {
    let same_concrete_type = a.type_id() == b.type_id() && !a.is_dynamic() && !b.is_dynamic();
    if same_concrete_type || a.type_name() == b.type_name() {
        // Fast path for the common case of two concrete/proxy values representing the same type
        let info = a.get_represented_type_info();
        return (info, info);
    }

    (a.get_represented_type_info(), b.get_represented_type_info())
}
