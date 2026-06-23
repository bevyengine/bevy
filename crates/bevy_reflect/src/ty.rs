//! Provides access to [`Type`]: a [`TypePath`]-powered replacement for [`TypeId`].

use crate::{TypePath, TypePathTable};
use core::any::{Any, TypeId};
use core::fmt::{Debug, Formatter};
use core::hash::Hash;

/// The base representation of a Rust type.
///
/// When possible, it is recommended to use [`&'static TypeInfo`] instead of this
/// as it provides more information as well as being smaller
/// (since a reference only takes the same number of bytes as a `usize`).
///
/// However, where a static reference to [`TypeInfo`] is not possible,
/// such as with trait objects and other types that can't implement [`Typed`],
/// this type can be used instead.
///
/// It only requires that the type implements [`TypePath`].
///
/// And unlike [`TypeInfo`], this type implements [`Copy`], [`Eq`], and [`Hash`],
/// making it useful as a key type.
///
/// It's especially helpful when compared to [`TypeId`] as it can provide the
/// actual [type path] when debugging, while still having the same performance
/// as hashing/comparing [`TypeId`] directly—at the cost of a little more memory.
///
/// # Examples
///
/// ```
/// use bevy_reflect::{Type, TypePath};
///
/// fn assert_char<T: ?Sized + TypePath>(t: &T) -> Result<(), String> {
///     let ty = Type::of::<T>();
///     if Type::of::<char>() == ty {
///         Ok(())
///     } else {
///         Err(format!("expected `char`, got `{}`", ty.path()))
///     }
/// }
///
/// assert_eq!(
///     assert_char(&'a'),
///     Ok(())
/// );
/// assert_eq!(
///     assert_char(&String::from("Hello, world!")),
///     Err(String::from("expected `char`, got `alloc::string::String`"))
/// );
/// ```
///
/// [`&'static TypeInfo`]: crate::info::TypeInfo
/// [`TypeInfo`]: crate::info::TypeInfo
/// [`Typed`]: crate::info::Typed
#[derive(Copy, Clone)]
pub struct Type {
    type_path_table: TypePathTable,
    type_id: TypeId,
}

impl Type {
    /// Create a new [`Type`] from a type that implements [`TypePath`].
    pub fn of<T: TypePath + ?Sized>() -> Self {
        Self {
            type_path_table: TypePathTable::of::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }

    /// Returns the [`TypeId`] of the type.
    #[inline]
    pub fn id(&self) -> TypeId {
        self.type_id
    }

    /// See [`crate::type_path::TypePath::type_path`].
    pub fn path(&self) -> &'static str {
        self.type_path_table.path()
    }

    /// See [`crate::type_path::TypePath::short_type_path`].
    pub fn short_path(&self) -> &'static str {
        self.type_path_table.short_path()
    }

    /// See [`crate::type_path::TypePath::type_ident`].
    pub fn ident(&self) -> Option<&'static str> {
        self.type_path_table.ident()
    }

    /// See [`crate::type_path::TypePath::crate_name`].
    pub fn crate_name(&self) -> Option<&'static str> {
        self.type_path_table.crate_name()
    }

    /// See [`crate::type_path::TypePath::module_path`].
    pub fn module_path(&self) -> Option<&'static str> {
        self.type_path_table.module_path()
    }

    /// A representation of the type path of this.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path_table
    }

    /// Check if the given type matches this one.
    ///
    /// This only compares the [`TypeId`] of the types
    /// and does not verify they share the same [`TypePath`]
    /// (though it implies they do).
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }
}

/// This implementation will only output the [type path] of the type.
///
/// If you need to include the [`TypeId`] in the output,
/// you can access it through [`Type::id`].
///
/// [type path]: TypePath
impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.type_path_table.path())
    }
}

impl Eq for Type {}

/// This implementation purely relies on the [`TypeId`] of the type,
/// and not on the [type path].
///
/// [type path]: TypePath
impl PartialEq for Type {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

/// This implementation purely relies on the [`TypeId`] of the type,
/// and not on the [type path].
///
/// [type path]: TypePath
impl Hash for Type {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

macro_rules! impl_type_methods {
    // Generates the type methods based off a single field.
    ($field:ident) => {
        $crate::ty::impl_type_methods!(self => {
            &self.$field
        });
    };
    // Generates the type methods based off a custom expression.
    ($self:ident => $expr:expr) => {
        /// The underlying Rust [type].
        ///
        /// [type]: crate::ty::Type
        pub fn ty(&$self) -> &$crate::ty::Type {
            $expr
        }

        /// The [`TypeId`] of this type.
        ///
        /// [`TypeId`]: core::any::TypeId
        pub fn type_id(&self) -> ::core::any::TypeId {
            self.ty().id()
        }

        /// The [stable, full type path] of this type.
        ///
        /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
        ///
        /// [stable, full type path]: TypePath
        /// [`type_path_table`]: Self::type_path_table
        pub fn type_path(&self) -> &'static str {
            self.ty().path()
        }

        /// A representation of the type path of this type.
        ///
        /// Provides dynamic access to all methods on [`TypePath`].
        ///
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn type_path_table(&self) -> &$crate::type_path::TypePathTable {
            &self.ty().type_path_table()
        }

        /// Check if the given type matches this one.
        ///
        /// This only compares the [`TypeId`] of the types
        /// and does not verify they share the same [`TypePath`]
        /// (though it implies they do).
        ///
        /// [`TypeId`]: core::any::TypeId
        /// [`TypePath`]: crate::type_path::TypePath
        pub fn is<T: ::core::any::Any>(&self) -> bool {
            self.ty().is::<T>()
        }
    };
}

pub(crate) use impl_type_methods;
