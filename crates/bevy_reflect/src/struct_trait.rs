use crate::utility::NonGenericTypeInfoCell;
use crate::{
    DynamicInfo, NamedField, Reflect, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, Typed,
};
use bevy_utils::{Entry, HashMap};
use std::fmt::{Debug, Formatter};
use std::{
    any::{Any, TypeId},
    borrow::Cow,
    slice::Iter,
};

/// A reflected Rust regular struct type.
///
/// Implementors of this trait allow their fields to be addressed by name as
/// well as by index.
///
/// This trait is automatically implemented for `struct` types with named fields
/// when using `#[derive(Reflect)]`.
///
/// # Example
///
/// ```
/// use bevy_reflect::{Reflect, Struct};
///
/// #[derive(Reflect)]
/// struct Foo {
///     bar: String,
/// }
///
/// # fn main() {
/// let foo = Foo { bar: "Hello, world!".to_string() };
///
/// assert_eq!(foo.field_len(), 1);
/// assert_eq!(foo.name_at(0), Some("bar"));
///
/// let bar = foo.field("bar").unwrap();
/// assert_eq!(bar.downcast_ref::<String>(), Some(&"Hello, world!".to_string()));
/// # }
/// ```
pub trait Struct: Reflect {
    /// Returns a reference to the value of the field named `name` as a `&dyn
    /// Reflect`.
    fn field(&self, name: &str) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the value of the field named `name` as a
    /// `&mut dyn Reflect`.
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect>;

    /// Returns a reference to the value of the field with index `index` as a
    /// `&dyn Reflect`.
    fn field_at(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the value of the field with index `index`
    /// as a `&mut dyn Reflect`.
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;

    /// Returns the name of the field with index `index`.
    fn name_at(&self, index: usize) -> Option<&str>;

    /// Returns the number of fields in the struct.
    fn field_len(&self) -> usize;

    /// Returns an iterator over the values of the reflectable fields for this struct.
    fn iter_fields(&self) -> FieldIter;

    /// Clones the struct into a [`DynamicStruct`].
    fn clone_dynamic(&self) -> DynamicStruct;
}

/// A container for compile-time struct info.
#[derive(Clone, Debug)]
pub struct StructInfo {
    name: &'static str,
    type_name: &'static str,
    type_id: TypeId,
    fields: Box<[NamedField]>,
    field_names: Box<[&'static str]>,
    field_indices: HashMap<&'static str, usize>,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl StructInfo {
    /// Create a new [`StructInfo`].
    ///
    /// # Arguments
    ///
    /// * `name`: The name of this struct (_without_ generics or lifetimes)
    /// * `fields`: The fields of this struct in the order they are defined
    ///
    pub fn new<T: Reflect>(name: &'static str, fields: &[NamedField]) -> Self {
        let field_indices = fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.name(), index))
            .collect::<HashMap<_, _>>();

        let field_names = fields.iter().map(|field| field.name()).collect();

        Self {
            name,
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
            fields: fields.to_vec().into_boxed_slice(),
            field_names,
            field_indices,
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this struct.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// A slice containing the names of all fields in order.
    pub fn field_names(&self) -> &[&'static str] {
        &self.field_names
    }

    /// Get the field with the given name.
    pub fn field(&self, name: &str) -> Option<&NamedField> {
        self.field_indices
            .get(name)
            .map(|index| &self.fields[*index])
    }

    /// Get the field at the given index.
    pub fn field_at(&self, index: usize) -> Option<&NamedField> {
        self.fields.get(index)
    }

    /// Get the index of the field with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }

    /// Iterate over the fields of this struct.
    pub fn iter(&self) -> Iter<'_, NamedField> {
        self.fields.iter()
    }

    /// The total number of fields in this struct.
    pub fn field_len(&self) -> usize {
        self.fields.len()
    }

    /// The name of the struct.
    ///
    /// This does _not_ include any generics or lifetimes.
    ///
    /// For example, `foo::bar::Baz<'a, T>` would simply be `Baz`.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The [type name] of the struct.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the struct.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the struct type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The docstring of this struct, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }
}

/// An iterator over the field values of a struct.
pub struct FieldIter<'a> {
    pub(crate) struct_val: &'a dyn Struct,
    pub(crate) index: usize,
}

impl<'a> FieldIter<'a> {
    pub fn new(value: &'a dyn Struct) -> Self {
        FieldIter {
            struct_val: value,
            index: 0,
        }
    }
}

impl<'a> Iterator for FieldIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.struct_val.field_at(self.index);
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.struct_val.field_len();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for FieldIter<'a> {}

/// A convenience trait which combines fetching and downcasting of struct
/// fields.
///
/// # Example
///
/// ```
/// use bevy_reflect::{GetField, Reflect};
///
/// #[derive(Reflect)]
/// struct Foo {
///     bar: String,
/// }
///
/// # fn main() {
/// let mut foo = Foo { bar: "Hello, world!".to_string() };
///
/// foo.get_field_mut::<String>("bar").unwrap().truncate(5);
/// assert_eq!(foo.get_field::<String>("bar"), Some(&"Hello".to_string()));
/// # }
/// ```
pub trait GetField {
    /// Returns a reference to the value of the field named `name`, downcast to
    /// `T`.
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T>;

    /// Returns a mutable reference to the value of the field named `name`,
    /// downcast to `T`.
    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T>;
}

impl<S: Struct> GetField for S {
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T> {
        self.field(name).and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T> {
        self.field_mut(name)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

impl GetField for dyn Struct {
    fn get_field<T: Reflect>(&self, name: &str) -> Option<&T> {
        self.field(name).and_then(|value| value.downcast_ref::<T>())
    }

    fn get_field_mut<T: Reflect>(&mut self, name: &str) -> Option<&mut T> {
        self.field_mut(name)
            .and_then(|value| value.downcast_mut::<T>())
    }
}

/// A struct type which allows fields to be added at runtime.
#[derive(Default)]
pub struct DynamicStruct {
    name: String,
    fields: Vec<Box<dyn Reflect>>,
    field_names: Vec<Cow<'static, str>>,
    field_indices: HashMap<Cow<'static, str>, usize>,
}

impl DynamicStruct {
    /// Returns the type name of the struct.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the type name of the struct.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Inserts a field named `name` with value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert_boxed(&mut self, name: &str, value: Box<dyn Reflect>) {
        let name = Cow::Owned(name.to_string());
        match self.field_indices.entry(name) {
            Entry::Occupied(entry) => {
                self.fields[*entry.get()] = value;
            }
            Entry::Vacant(entry) => {
                self.fields.push(value);
                self.field_names.push(entry.key().clone());
                entry.insert(self.fields.len() - 1);
            }
        }
    }

    /// Inserts a field named `name` with the typed value `value` into the struct.
    ///
    /// If the field already exists, it is overwritten.
    pub fn insert<T: Reflect>(&mut self, name: &str, value: T) {
        if let Some(index) = self.field_indices.get(name) {
            self.fields[*index] = Box::new(value);
        } else {
            self.insert_boxed(name, Box::new(value));
        }
    }

    /// Gets the index of the field with the given name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.field_indices.get(name).copied()
    }
}

impl Struct for DynamicStruct {
    #[inline]
    fn field(&self, name: &str) -> Option<&dyn Reflect> {
        self.field_indices
            .get(name)
            .map(|index| &*self.fields[*index])
    }

    #[inline]
    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        if let Some(index) = self.field_indices.get(name) {
            Some(&mut *self.fields[*index])
        } else {
            None
        }
    }

    #[inline]
    fn field_at(&self, index: usize) -> Option<&dyn Reflect> {
        self.fields.get(index).map(|value| &**value)
    }

    #[inline]
    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.fields.get_mut(index).map(|value| &mut **value)
    }

    #[inline]
    fn name_at(&self, index: usize) -> Option<&str> {
        self.field_names.get(index).map(|name| name.as_ref())
    }

    #[inline]
    fn field_len(&self) -> usize {
        self.fields.len()
    }

    #[inline]
    fn iter_fields(&self) -> FieldIter {
        FieldIter {
            struct_val: self,
            index: 0,
        }
    }

    fn clone_dynamic(&self) -> DynamicStruct {
        DynamicStruct {
            name: self.name.clone(),
            field_names: self.field_names.clone(),
            field_indices: self.field_indices.clone(),
            fields: self
                .fields
                .iter()
                .map(|value| value.clone_value())
                .collect(),
        }
    }
}

impl Reflect for DynamicStruct {
    #[inline]
    fn type_name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Struct(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Struct(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Struct(self)
    }

    fn apply(&mut self, value: &dyn Reflect) {
        if let ReflectRef::Struct(struct_value) = value.reflect_ref() {
            for (i, value) in struct_value.iter_fields().enumerate() {
                let name = struct_value.name_at(i).unwrap();
                if let Some(v) = self.field_mut(name) {
                    v.apply(value);
                }
            }
        } else {
            panic!("Attempted to apply non-struct type to struct type.");
        }
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        struct_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicStruct(")?;
        struct_debug(self, f)?;
        write!(f, ")")
    }
}

impl Debug for DynamicStruct {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

impl Typed for DynamicStruct {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Dynamic(DynamicInfo::new::<Self>()))
    }
}

/// Compares a [`Struct`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a struct;
/// - For each field in `a`, `b` contains a field with the same name and
///   [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two field
///   values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn struct_partial_eq<S: Struct>(a: &S, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::Struct(struct_value) = b.reflect_ref()  else {
        return Some(false);
    };

    if a.field_len() != struct_value.field_len() {
        return Some(false);
    }

    for (i, value) in struct_value.iter_fields().enumerate() {
        let name = struct_value.name_at(i).unwrap();
        if let Some(field_value) = a.field(name) {
            let eq_result = field_value.reflect_partial_eq(value);
            if let failed @ (Some(false) | None) = eq_result {
                return failed;
            }
        } else {
            return Some(false);
        }
    }

    Some(true)
}

/// The default debug formatter for [`Struct`] types.
///
/// # Example
/// ```
/// use bevy_reflect::Reflect;
/// #[derive(Reflect)]
/// struct MyStruct {
///   foo: usize
/// }
///
/// let my_struct: &dyn Reflect = &MyStruct { foo: 123 };
/// println!("{:#?}", my_struct);
///
/// // Output:
///
/// // MyStruct {
/// //   foo: 123,
/// // }
/// ```
#[inline]
pub fn struct_debug(dyn_struct: &dyn Struct, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut debug = f.debug_struct(dyn_struct.type_name());
    for field_index in 0..dyn_struct.field_len() {
        let field = dyn_struct.field_at(field_index).unwrap();
        debug.field(
            dyn_struct.name_at(field_index).unwrap(),
            &field as &dyn Debug,
        );
    }
    debug.finish()
}
