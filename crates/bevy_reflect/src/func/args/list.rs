use crate::{
    func::{
        args::{Arg, ArgValue, FromArg},
        ArgError,
    },
    PartialReflect, Reflect, TypePath,
};
use alloc::{
    boxed::Box,
    collections::vec_deque::{Iter, VecDeque},
};

/// A list of arguments that can be passed to a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{ArgValue, ArgList};
/// let foo = 123;
/// let bar = 456;
/// let mut baz = 789;
/// let args = ArgList::new()
///   // Push an owned argument
///   .with_owned(foo)
///   // Push an owned and boxed argument
///   .with_boxed(Box::new(foo))
///   // Push a reference argument
///   .with_ref(&bar)
///   // Push a mutable reference argument
///   .with_mut(&mut baz)
///   // Push a manually constructed argument
///   .with_arg(ArgValue::Ref(&3.14));
/// ```
///
/// [arguments]: Arg
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Default, Debug)]
pub struct ArgList<'a> {
    list: VecDeque<Arg<'a>>,
    /// A flag that indicates if the list needs to be re-indexed.
    ///
    /// This flag should be set when an argument is removed from the beginning of the list,
    /// so that any future push operations will re-index the arguments.
    needs_reindex: bool,
}

impl<'a> ArgList<'a> {
    /// Create a new empty list of arguments.
    pub fn new() -> Self {
        Self {
            list: VecDeque::new(),
            needs_reindex: false,
        }
    }

    /// Push an [`ArgValue`] onto the list.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn push_arg(&mut self, arg: ArgValue<'a>) {
        if self.needs_reindex {
            for (index, arg) in self.list.iter_mut().enumerate() {
                arg.set_index(index);
            }
            self.needs_reindex = false;
        }

        let index = self.list.len();
        self.list.push_back(Arg::new(index, arg));
    }

    /// Push an [`ArgValue::Ref`] onto the list with the given reference.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn push_ref(&mut self, arg: &'a dyn PartialReflect) {
        self.push_arg(ArgValue::Ref(arg));
    }

    /// Push an [`ArgValue::Mut`] onto the list with the given mutable reference.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn push_mut(&mut self, arg: &'a mut dyn PartialReflect) {
        self.push_arg(ArgValue::Mut(arg));
    }

    /// Push an [`ArgValue::Owned`] onto the list with the given owned value.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn push_owned(&mut self, arg: impl PartialReflect) {
        self.push_arg(ArgValue::Owned(Box::new(arg)));
    }

    /// Push an [`ArgValue::Owned`] onto the list with the given boxed value.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn push_boxed(&mut self, arg: Box<dyn PartialReflect>) {
        self.push_arg(ArgValue::Owned(arg));
    }

    /// Push an [`ArgValue`] onto the list.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn with_arg(mut self, arg: ArgValue<'a>) -> Self {
        self.push_arg(arg);
        self
    }

    /// Push an [`ArgValue::Ref`] onto the list with the given reference.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn with_ref(self, arg: &'a dyn PartialReflect) -> Self {
        self.with_arg(ArgValue::Ref(arg))
    }

    /// Push an [`ArgValue::Mut`] onto the list with the given mutable reference.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn with_mut(self, arg: &'a mut dyn PartialReflect) -> Self {
        self.with_arg(ArgValue::Mut(arg))
    }

    /// Push an [`ArgValue::Owned`] onto the list with the given owned value.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn with_owned(self, arg: impl PartialReflect) -> Self {
        self.with_arg(ArgValue::Owned(Box::new(arg)))
    }

    /// Push an [`ArgValue::Owned`] onto the list with the given boxed value.
    ///
    /// If an argument was previously removed from the beginning of the list,
    /// this method will also re-index the list.
    pub fn with_boxed(self, arg: Box<dyn PartialReflect>) -> Self {
        self.with_arg(ArgValue::Owned(arg))
    }

    /// Remove the first argument in the list and return it.
    ///
    /// It's generally preferred to use [`Self::take`] instead of this method
    /// as it provides a more ergonomic way to immediately downcast the argument.
    pub fn take_arg(&mut self) -> Result<Arg<'a>, ArgError> {
        self.needs_reindex = true;
        self.list.pop_front().ok_or(ArgError::EmptyArgList)
    }

    /// Remove the first argument in the list and return `Ok(T::This)`.
    ///
    /// If the list is empty or the [`FromArg::from_arg`] call fails, returns an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let a = 1u32;
    /// let b = 2u32;
    /// let mut c = 3u32;
    /// let mut args = ArgList::new().with_owned(a).with_ref(&b).with_mut(&mut c);
    ///
    /// let a = args.take::<u32>().unwrap();
    /// assert_eq!(a, 1);
    ///
    /// let b = args.take::<&u32>().unwrap();
    /// assert_eq!(*b, 2);
    ///
    /// let c = args.take::<&mut u32>().unwrap();
    /// assert_eq!(*c, 3);
    /// ```
    pub fn take<T: FromArg>(&mut self) -> Result<T::This<'a>, ArgError> {
        self.take_arg()?.take::<T>()
    }

    /// Remove the first argument in the list and return `Ok(T)` if the argument is [`ArgValue::Owned`].
    ///
    /// If the list is empty or the argument is not owned, returns an error.
    ///
    /// It's generally preferred to use [`Self::take`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().with_owned(value);
    /// let value = args.take_owned::<u32>().unwrap();
    /// assert_eq!(value, 123);
    /// ```
    pub fn take_owned<T: Reflect + TypePath>(&mut self) -> Result<T, ArgError> {
        self.take_arg()?.take_owned()
    }

    /// Remove the first argument in the list and return `Ok(&T)` if the argument is [`ArgValue::Ref`].
    ///
    /// If the list is empty or the argument is not a reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::take`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().with_ref(&value);
    /// let value = args.take_ref::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn take_ref<T: Reflect + TypePath>(&mut self) -> Result<&'a T, ArgError> {
        self.take_arg()?.take_ref()
    }

    /// Remove the first argument in the list and return `Ok(&mut T)` if the argument is [`ArgValue::Mut`].
    ///
    /// If the list is empty or the argument is not a mutable reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::take`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let mut value = 123u32;
    /// let mut args = ArgList::new().with_mut(&mut value);
    /// let value = args.take_mut::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn take_mut<T: Reflect + TypePath>(&mut self) -> Result<&'a mut T, ArgError> {
        self.take_arg()?.take_mut()
    }

    /// Remove the last argument in the list and return it.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method
    /// as it provides a more ergonomic way to immediately downcast the argument.
    pub fn pop_arg(&mut self) -> Result<Arg<'a>, ArgError> {
        self.list.pop_back().ok_or(ArgError::EmptyArgList)
    }

    /// Remove the last argument in the list and return `Ok(T::This)`.
    ///
    /// If the list is empty or the [`FromArg::from_arg`] call fails, returns an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let a = 1u32;
    /// let b = 2u32;
    /// let mut c = 3u32;
    /// let mut args = ArgList::new().with_owned(a).with_ref(&b).with_mut(&mut c);
    ///
    /// let c = args.pop::<&mut u32>().unwrap();
    /// assert_eq!(*c, 3);
    ///
    /// let b = args.pop::<&u32>().unwrap();
    /// assert_eq!(*b, 2);
    ///
    /// let a = args.pop::<u32>().unwrap();
    /// assert_eq!(a, 1);
    /// ```
    pub fn pop<T: FromArg>(&mut self) -> Result<T::This<'a>, ArgError> {
        self.pop_arg()?.take::<T>()
    }

    /// Remove the last argument in the list and return `Ok(T)` if the argument is [`ArgValue::Owned`].
    ///
    /// If the list is empty or the argument is not owned, returns an error.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().with_owned(value);
    /// let value = args.pop_owned::<u32>().unwrap();
    /// assert_eq!(value, 123);
    /// ```
    pub fn pop_owned<T: Reflect + TypePath>(&mut self) -> Result<T, ArgError> {
        self.pop_arg()?.take_owned()
    }

    /// Remove the last argument in the list and return `Ok(&T)` if the argument is [`ArgValue::Ref`].
    ///
    /// If the list is empty or the argument is not a reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let value = 123u32;
    /// let mut args = ArgList::new().with_ref(&value);
    /// let value = args.pop_ref::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn pop_ref<T: Reflect + TypePath>(&mut self) -> Result<&'a T, ArgError> {
        self.pop_arg()?.take_ref()
    }

    /// Remove the last argument in the list and return `Ok(&mut T)` if the argument is [`ArgValue::Mut`].
    ///
    /// If the list is empty or the argument is not a mutable reference, returns an error.
    ///
    /// It's generally preferred to use [`Self::pop`] instead of this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::ArgList;
    /// let mut value = 123u32;
    /// let mut args = ArgList::new().with_mut(&mut value);
    /// let value = args.pop_mut::<u32>().unwrap();
    /// assert_eq!(*value, 123);
    /// ```
    pub fn pop_mut<T: Reflect + TypePath>(&mut self) -> Result<&'a mut T, ArgError> {
        self.pop_arg()?.take_mut()
    }

    /// Returns an iterator over the arguments in the list.
    pub fn iter(&self) -> Iter<'_, Arg<'a>> {
        self.list.iter()
    }

    /// Returns the number of arguments in the list.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Returns `true` if the list of arguments is empty.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn should_push_arguments_in_order() {
        let args = ArgList::new()
            .with_owned(123)
            .with_owned(456)
            .with_owned(789);

        assert_eq!(args.len(), 3);
        assert_eq!(args.list[0].index(), 0);
        assert_eq!(args.list[1].index(), 1);
        assert_eq!(args.list[2].index(), 2);
    }

    #[test]
    fn should_push_arg_with_correct_ownership() {
        let a = String::from("a");
        let b = String::from("b");
        let mut c = String::from("c");
        let d = String::from("d");
        let e = String::from("e");
        let f = String::from("f");
        let mut g = String::from("g");

        let args = ArgList::new()
            .with_arg(ArgValue::Owned(Box::new(a)))
            .with_arg(ArgValue::Ref(&b))
            .with_arg(ArgValue::Mut(&mut c))
            .with_owned(d)
            .with_boxed(Box::new(e))
            .with_ref(&f)
            .with_mut(&mut g);

        assert!(matches!(args.list[0].value(), &ArgValue::Owned(_)));
        assert!(matches!(args.list[1].value(), &ArgValue::Ref(_)));
        assert!(matches!(args.list[2].value(), &ArgValue::Mut(_)));
        assert!(matches!(args.list[3].value(), &ArgValue::Owned(_)));
        assert!(matches!(args.list[4].value(), &ArgValue::Owned(_)));
        assert!(matches!(args.list[5].value(), &ArgValue::Ref(_)));
        assert!(matches!(args.list[6].value(), &ArgValue::Mut(_)));
    }

    #[test]
    fn should_take_args_in_order() {
        let a = String::from("a");
        let b = 123_i32;
        let c = 456_usize;
        let mut d = 5.78_f32;

        let mut args = ArgList::new()
            .with_owned(a)
            .with_ref(&b)
            .with_ref(&c)
            .with_mut(&mut d);

        assert_eq!(args.len(), 4);
        assert_eq!(args.take_owned::<String>().unwrap(), String::from("a"));
        assert_eq!(args.take::<&i32>().unwrap(), &123);
        assert_eq!(args.take_ref::<usize>().unwrap(), &456);
        assert_eq!(args.take_mut::<f32>().unwrap(), &mut 5.78);
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn should_pop_args_in_reverse_order() {
        let a = String::from("a");
        let b = 123_i32;
        let c = 456_usize;
        let mut d = 5.78_f32;

        let mut args = ArgList::new()
            .with_owned(a)
            .with_ref(&b)
            .with_ref(&c)
            .with_mut(&mut d);

        assert_eq!(args.len(), 4);
        assert_eq!(args.pop_mut::<f32>().unwrap(), &mut 5.78);
        assert_eq!(args.pop_ref::<usize>().unwrap(), &456);
        assert_eq!(args.pop::<&i32>().unwrap(), &123);
        assert_eq!(args.pop_owned::<String>().unwrap(), String::from("a"));
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn should_reindex_on_push_after_take() {
        let mut args = ArgList::new()
            .with_owned(123)
            .with_owned(456)
            .with_owned(789);

        assert!(!args.needs_reindex);

        args.take_arg().unwrap();
        assert!(args.needs_reindex);
        assert!(args.list[0].value().reflect_partial_eq(&456).unwrap());
        assert_eq!(args.list[0].index(), 1);
        assert!(args.list[1].value().reflect_partial_eq(&789).unwrap());
        assert_eq!(args.list[1].index(), 2);

        let args = args.with_owned(123);
        assert!(!args.needs_reindex);
        assert!(args.list[0].value().reflect_partial_eq(&456).unwrap());
        assert_eq!(args.list[0].index(), 0);
        assert!(args.list[1].value().reflect_partial_eq(&789).unwrap());
        assert_eq!(args.list[1].index(), 1);
        assert!(args.list[2].value().reflect_partial_eq(&123).unwrap());
        assert_eq!(args.list[2].index(), 2);
    }
}
