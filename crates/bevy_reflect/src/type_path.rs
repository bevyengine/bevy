pub trait TypePath: 'static {
    /// Returns the fully qualified path of the underlying type.
    ///
    /// For [`Option<()>`], this is `core::option::Option::<()>`. 
    fn type_path() -> &'static str;
    
    /// Returns a short pretty-print enabled path to the type.
    ///
    /// For [`Option<()>`], this is `Option<()>`. 
    fn short_type_path() -> &'static str;
    
    /// Returns the name of the type, or [`None`] if it is anonymous.
    ///
    /// For [`Option<()>`], this is `Option`. 
    fn type_ident() -> Option<&'static str>;
    
    /// Returns the name of the crate the type is in, or [`None`] if it is anonymous or a primitive.
    ///
    /// For [`Option<()>`], this is `core`. 
    fn crate_name() -> Option<&'static str>;
    
    /// Returns the path to the moudle the type is in, or [`None`] if it is anonymous or a primitive.
    ///
    /// For [`Option<()>`], this is `core::option`. 
    fn module_path() -> Option<&'static str>;
}