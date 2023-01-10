pub trait WithPath {
    /// Returns the [path] of the underlying type.
    ///
    /// [path]: TypePath
    fn type_path() -> &'static TypePath;
}

pub struct TypePath {
    path: String,
    short_path: String,
    ident: Option<String>,
    crate_name: Option<String>,
    module_path: Option<String>,
}

impl TypePath {
    pub fn new_primitive(name: String) -> Self {
        Self {
            path: name.clone(),
            short_path: name.clone(),
            ident: Some(name),
            crate_name: None,
            module_path: None,
            
        }
    }
    
    pub fn new_anonymous(path: String, short_path: String) -> Self {
        Self {
            path,
            short_path,
            ident: None,
            crate_name: None,
            module_path: None,
        }
    }
    
    pub fn new_named(path: String, short_path: String, ident: String, crate_name: String, module_path: String) -> Self {
        Self {
            path,
            short_path,
            ident: Some(ident),
            crate_name: Some(crate_name),
            module_path: Some(module_path),
        }
    }
    
    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[inline]
    pub fn short_path(&self) -> &str {
        &self.short_path
    }

    #[inline]
    pub fn ident(&self) -> Option<&str> {
        self.ident.as_deref()
    }

    #[inline]
    pub fn crate_name(&self) -> Option<&str> {
        self.crate_name.as_deref()
    }

    #[inline]
    pub fn module_path(&self) -> Option<&str> {
        self.module_path.as_deref()
    }
}