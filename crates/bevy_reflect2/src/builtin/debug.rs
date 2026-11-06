use crate::{FieldKind, KindRef, Reflect, StructKind};
use core::fmt::{self, Debug, Formatter};

impl dyn Reflect {
    /// Formats this value using its [`Debug`] implementation, falling back to a structural
    /// implementation if it doesn't have one. Values that cannot be formatted structurally print
    /// their type path.
    ///
    /// [`Debug`] is also implemented for `dyn Reflect` using this method.
    pub fn reflect_debug(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let ty = self.ty();
        if let Some(debug) = ty.vtable().debug {
            // SAFETY: `Reflect::ty` accurately describes this value
            return unsafe { debug(self.ptr(), f) };
        }
        match self.kind_ref() {
            KindRef::Struct(value) => {
                // Tuples are formatted like their `Debug` implementation (ex: `(1, 2.0)`). An empty
                // name formats like a tuple, including the trailing comma of 1-tuples.
                let name = match value.struct_type().struct_kind() {
                    StructKind::Struct => ty.type_path().short(),
                    StructKind::Tuple => "",
                };
                match value.struct_type().field_kind() {
                    FieldKind::Named => {
                        let mut debug = f.debug_struct(name);
                        for (field, value) in value.iter() {
                            debug.field(field.name, &value);
                        }
                        if value.struct_type().has_ignored_fields() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unnamed => {
                        let mut debug = f.debug_tuple(name);
                        for (_, value) in value.iter() {
                            debug.field(&value);
                        }
                        if value.struct_type().has_ignored_fields() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unit => f.write_str(name),
                }
            }
            KindRef::Enum(value) => {
                let variant = value.variant_type();
                match variant.field_kind {
                    FieldKind::Named => {
                        let mut debug = f.debug_struct(variant.name);
                        for (field, value) in value.iter() {
                            debug.field(field.name, &value);
                        }
                        if !variant.ignored_fields.is_empty() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unnamed => {
                        let mut debug = f.debug_tuple(variant.name);
                        for (_, value) in value.iter() {
                            debug.field(&value);
                        }
                        if !variant.ignored_fields.is_empty() {
                            debug.finish_non_exhaustive()
                        } else {
                            debug.finish()
                        }
                    }
                    FieldKind::Unit => f.write_str(variant.name),
                }
            }
            KindRef::List(value) => f.debug_list().entries(value.iter()).finish(),
            KindRef::Array(value) => f.debug_list().entries(value.iter()).finish(),
            KindRef::Map(value) => f.debug_map().entries(value.iter()).finish(),
            KindRef::Set(value) => f.debug_set().entries(value.iter()).finish(),
            KindRef::Primitive(_) => write!(f, "Reflect({})", ty.type_path().full()),
        }
    }
}

impl Debug for dyn Reflect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.reflect_debug(f)
    }
}
