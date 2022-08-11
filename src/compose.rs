use std::collections::{BTreeSet, HashMap, HashSet};

use naga::{valid::ValidationError, EntryPoint, WithSpan};
use regex::Regex;

use crate::{derive::DerivedModule, util::clone_module};

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ShaderLanguage {
    Wgsl,
}

// a module built with a specific set of shader_defs
#[derive(Default, Debug)]
pub struct ComposableModule {
    // imports required in the current configuration
    pub imports: Vec<String>,
    // types exported
    pub owned_types: HashSet<String>,
    // constants exported
    pub owned_constants: HashSet<String>,
    // vars exported
    pub owned_vars: HashSet<String>,
    // functions exported
    pub owned_functions: HashSet<String>,
    // naga module, built against headers for any imports
    module_ir: naga::Module,
    // headers in different shader languages, used for building modules/shaders that import this module
    // headers contain types, constants, global vars and empty function definitions - 
    // just enough to convert source strings that want to import this module into naga IR
    headers: HashMap<ShaderLanguage, String>,
}

// data used to build a ComposableModule
#[derive(Debug)]
pub struct ComposableModuleDefinition {
    pub name: String,
    // shader text
    pub source: String,
    // language
    pub language: ShaderLanguage,
    // list of shader_defs that can affect this module
    effective_defs: Vec<String>,
    // built composable modules for a given set of shader defs
    modules: HashMap<Vec<String>, ComposableModule>,
}

impl ComposableModuleDefinition {
    fn get(&self, shader_defs: &HashSet<String>) -> Option<&ComposableModule> {
        let mut effective_defs: Vec<_> = self
            .effective_defs
            .iter()
            .filter(|&def| shader_defs.contains(def))
            .cloned()
            .collect();
        effective_defs.sort();
        self.modules.get(&effective_defs)
    }

    fn insert(
        &mut self,
        shader_defs: &HashSet<String>,
        module: ComposableModule,
    ) -> &ComposableModule {
        let mut effective_defs: Vec<_> = self
            .effective_defs
            .iter()
            .filter(|&def| shader_defs.contains(def))
            .cloned()
            .collect();
        effective_defs.sort();
        self.modules.insert(effective_defs.clone(), module);
        self.modules.get(&effective_defs).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct ImportDefinition {
    pub import: String,
    pub as_name: String,
}

#[derive(Debug)]
pub enum ComposerError {
    ImportNotFound(String),
    WgslParseError(naga::front::wgsl::ParseError, String),
    WgslBackError(naga::back::wgsl::Error, naga::Module),
    HeaderValidationError(naga::WithSpan<naga::valid::ValidationError>, naga::Module),
    ShaderValidationError(naga::WithSpan<naga::valid::ValidationError>, naga::Module),
    NotEnoughEndIfs(String),
    TooManyEndIfs(String),
    NoModuleName,
}

// module composer
// stores any modules that can be imported into a shader
// and builds the final shader
#[derive(Debug)]
pub struct Composer {
    pub validate: bool,
    pub module_sets: HashMap<String, ComposableModuleDefinition>,
    ifdef_regex: Regex,
    ifndef_regex: Regex,
    else_regex: Regex,
    endif_regex: Regex,
    import_custom_path_as_regex: Regex,
    import_custom_path_regex: Regex,
    define_import_path_regex: Regex,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            validate: true,
            module_sets: Default::default(),
            ifdef_regex: Regex::new(r"^\s*#\s*ifdef\s+([\w|\d|_]+)").unwrap(),
            ifndef_regex: Regex::new(r"^\s*#\s*ifndef\s+([\w|\d|_]+)").unwrap(),
            else_regex: Regex::new(r"^\s*#\s*else").unwrap(),
            endif_regex: Regex::new(r"^\s*#\s*endif").unwrap(),
            import_custom_path_as_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)\s+as\s+([^\s]+)")
                .unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)").unwrap(),
            define_import_path_regex: Regex::new(r"^\s*#\s*define_import_path\s+(.+)").unwrap(),
        }
    }
}

const DECORATION_PRE: &str = "_naga_oil_mod__";
const DECORATION_POST: &str = "__";

fn decorate(as_name: &str) -> String {
    // todo check for collisions ("a/b" == "a\b" etc)
    let as_name = as_name.replace(|c: char| !c.is_alphanumeric(), "_");
    format!("{}{}{}", DECORATION_PRE, as_name, DECORATION_POST)
}

impl Composer {
    pub fn non_validating() -> Self {
        Self {
            validate: false,
            ..Default::default()
        }
    }

    // recursively add header strings to the input string for each imported module
    // requires that imports are already built
    fn add_header_strings<'a>(
        &'a self,
        lang: ShaderLanguage,
        module_string: &mut String,
        imports: &'a Vec<String>,
        already_added: &mut HashSet<&'a String>,
        shader_defs: &HashSet<String>,
    ) {
        for import in imports {
            if already_added.contains(import) {
                continue;
            }
            already_added.insert(import);

            // we must have ensured these exist with Composer::ensure_imports()
            let import_module_set = self.module_sets.get(import).unwrap();
            let module = import_module_set.get(shader_defs).unwrap();

            self.add_header_strings(
                lang,
                module_string,
                &module.imports,
                already_added,
                shader_defs,
            );
            module_string.push_str(module.headers.get(&lang).unwrap().as_str());
        }
    }

    // build naga module for a given shader_def configuration. builds a minimal self-contained module built against headers for imports
    fn create_module_ir(
        &mut self,
        source: String,
        language: ShaderLanguage,
        imports: &[ImportDefinition],
        shader_defs: &HashSet<String>,
    ) -> Result<naga::Module, ComposerError> {
        let mut module_string = String::new();
        self.add_header_strings(
            language,
            &mut module_string,
            &imports.iter().map(|def| &def.import).cloned().collect(),
            &mut Default::default(),
            shader_defs,
        );

        // sort imports by decreasing length so we don't accidentally replace substrings of a longer import
        let mut imports: Vec<_> = imports.iter().collect();
        imports.sort_by_key(|import| usize::MAX - import.as_name.len());

        let mut source = source;
        for ImportDefinition { import, as_name } in imports.iter() {
            // println!("replacing {} -> {}", format!("{}::", as_name), &decorate(import));
            source = source.replace(format!("{}::", as_name).as_str(), &decorate(import));
        }

        module_string.push_str(&source);

        // println!("parsing: {}", module_string);
        match language {
            ShaderLanguage::Wgsl => naga::front::wgsl::parse_str(module_string.as_str())
                .map_err(|e| ComposerError::WgslParseError(e, module_string)),                
        }
    }

    // process #if(n)def / #else / #endif preprocessor directives,
    // strip module name and imports
    fn preprocess_defs(
        &self,
        mod_name: &str,
        shader_str: &str,
        shader_defs: &HashSet<String>,
    ) -> Result<(String, Vec<ImportDefinition>), ComposerError> {
        let mut imports = Vec::new();
        let mut scopes = vec![true];
        let mut final_string = String::new();

        // this code broadly stolen from bevy_render::ShaderProcessor
        for line in shader_str.lines() {
            let mut output = false;
            if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && shader_defs.contains(def.as_str()));
            } else if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && !shader_defs.contains(def.as_str()));
            } else if self.else_regex.is_match(line) {
                let mut is_parent_scope_truthy = true;
                if scopes.len() > 1 {
                    is_parent_scope_truthy = scopes[scopes.len() - 2];
                }
                if let Some(last) = scopes.last_mut() {
                    *last = is_parent_scope_truthy && !*last;
                }
            } else if self.endif_regex.is_match(line) {
                scopes.pop();
                if scopes.is_empty() {
                    return Err(ComposerError::TooManyEndIfs(mod_name.to_owned()));
                }
            } else if self.define_import_path_regex.is_match(line) {
                // skip
            } else if *scopes.last().unwrap() {
                if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                    imports.push(ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: cap.get(2).unwrap().as_str().to_string(),
                    });
                } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                    imports.push(ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: cap.get(1).unwrap().as_str().to_string(),
                    });
                } else {
                    final_string.push_str(line);
                    output = true;
                }
            }

            if !output {
                final_string.extend(std::iter::repeat(" ").take(line.len()));
            }
            final_string.push('\n');
        }

        if scopes.len() != 1 {
            return Err(ComposerError::NotEnoughEndIfs(mod_name.to_owned()));
        }

        Ok((final_string, imports))
    }

    // extract module name and imports
    fn get_preprocessor_data(
        &self,
        shader_str: &str,
    ) -> (Option<String>, Vec<ImportDefinition>) {
        let mut imports = Vec::new();
        let mut name = None;
        for line in shader_str.lines() {
            if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                imports.push(ImportDefinition {
                    import: cap.get(1).unwrap().as_str().to_string(),
                    as_name: cap.get(2).unwrap().as_str().to_string(),
                });
            } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                imports.push(ImportDefinition {
                    import: cap.get(1).unwrap().as_str().to_string(),
                    as_name: cap.get(1).unwrap().as_str().to_string(),
                });
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                name = Some(cap.get(1).unwrap().as_str().to_string());
            }
        }
        (name, imports)
    }

    // build a ComposableModule from a ComposableModuleDefinition, for a given set of shader defs
    // - build the naga IR (against headers)
    // - record any types/vars/constants/functions that are defined within this module
    // - build headers for each supported language
    fn create_composable_module(
        &mut self,
        module_decoration: &str,
        unprocessed_source: &str,
        language: ShaderLanguage,
        shader_defs: &HashSet<String>,
    ) -> Result<ComposableModule, ComposerError> {
        let (source, imports) =
            self.preprocess_defs(module_decoration, unprocessed_source, shader_defs)?;
        let mut source_ir = self.create_module_ir(source, language, &imports, shader_defs)?;

        // rename and record owned items (except types which can't be mutably accessed)
        let mut owned_constants = HashMap::new();
        for (h, c) in source_ir.constants.iter_mut() {
            if let Some(name) = c.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{}{}", module_decoration, name);
                    owned_constants.insert(name.clone(), h);
                }
            }
        }

        let mut owned_vars = HashMap::new();
        for (h, gv) in source_ir.global_variables.iter_mut() {
            if let Some(name) = gv.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{}{}", module_decoration, name);

                    owned_vars.insert(name.clone(), h);
                }
            }
        }

        let mut owned_functions = HashMap::new();
        for (_, f) in source_ir.functions.iter_mut() {
            if let Some(name) = f.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{}{}", module_decoration, name);

                    let header_function = naga::Function {
                        name: Some(name.clone()),
                        arguments: f.arguments.to_vec(),
                        result: f.result.clone(),
                        local_variables: Default::default(),
                        expressions: Default::default(),
                        named_expressions: Default::default(),
                        body: Default::default(),
                    };
                    owned_functions.insert(name.clone(), header_function);
                }
            }
        }

        let mut module_builder = DerivedModule::default();
        let mut header_builder = DerivedModule::default();
        module_builder.set_shader_source(&source_ir);
        header_builder.set_shader_source(&source_ir);

        let mut owned_types = HashSet::new();
        for (h, ty) in source_ir.types.iter() {
            if let Some(name) = &ty.name {
                if !name.contains(DECORATION_PRE) {
                    let name = format!("{}{}", module_decoration, name);
                    owned_types.insert(name.clone());
                    // copy and rename types
                    module_builder.rename_type(&h, Some(name.clone()));
                    header_builder.rename_type(&h, Some(name));
                    continue;
                }
            }

            // copy all required types
            module_builder.import_type(&h);
        }

        // copy owned types into header and module
        for (_, h) in owned_constants.iter() {
            header_builder.import_const(h);
            module_builder.import_const(h);
        }

        for (_, h) in owned_vars.iter() {
            header_builder.import_global(h);
            module_builder.import_global(h);
        }

        // only stubs of owned functions into the header
        for (_, f) in owned_functions.iter() {
            header_builder.import_function(f); // header stub function
        }
        // all functions into the module (note source_ir only contains stubs for imported functions)
        for (_, f) in source_ir.functions.iter() {
            module_builder.import_function(f);
        }

        let header_ir = header_builder.into();
        // println!("[{}] header ir: \n{:#?}", module_decoration, header_ir);
        // println!("[{}] owned : \n{:#?}", module_decoration, (&owned_types, &owned_constants, &owned_vars, &owned_functions));
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&header_ir);

        let info = match info {
            Ok(info) => info,
            Err(e) => return Err(ComposerError::HeaderValidationError(e, clone_module(&header_ir))),
        };
        let headers = [(
            ShaderLanguage::Wgsl,
            naga::back::wgsl::write_string(
                &header_ir,
                &info,
                naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
            )
            .map_err(|e| ComposerError::WgslBackError(e, clone_module(&header_ir)))?,
        )]
        .into();

        let entry_points = source_ir.entry_points.iter().map(|ep| {
            EntryPoint {
                name: ep.name.clone(),
                stage: ep.stage,
                early_depth_test: ep.early_depth_test,
                workgroup_size: ep.workgroup_size,
                function: module_builder.localize_function(&ep.function),
            }
        }).collect();

        let module_ir = naga::Module {
            entry_points,
            ..module_builder.into()
        };        

        // println!("created header for {}: {:?}", module_decoration, headers);

        let composable_module = ComposableModule {
            imports: imports.into_iter().map(|def| def.import).collect(),
            owned_types,
            owned_constants: owned_constants.into_iter().map(|(n, _)| n).collect(),
            owned_vars: owned_vars.into_iter().map(|(n, _)| n).collect(),
            owned_functions: owned_functions.into_iter().map(|(n, _)| n).collect(),
            module_ir,
            headers,
        };

        Ok(composable_module)
    }

    // add a composable module to the composer
    // all imported modules must already have been added
    pub fn add_composable_module(
        &mut self,
        source: String,
        language: ShaderLanguage,
    ) -> Result<&ComposableModuleDefinition, ComposerError> {
        // todo reject a module containing the DECORATION_PRE string

        // use btreeset so the result is sorted
        let mut effective_defs = BTreeSet::new();

        let (module_name, imports) = self.get_preprocessor_data(&source);

        if module_name.is_none() {
            return Err(ComposerError::NoModuleName);
        }
        let module_name = module_name.unwrap();

        for import in imports.iter() {
            // we require modules already added so that we can capture the shader_defs that may impact us by impacting our dependencies
            let module_set = self
                .module_sets
                .get(&import.import)
                .ok_or_else(|| ComposerError::ImportNotFound(import.import.clone()))?;
            effective_defs.extend(module_set.effective_defs.iter().map(String::as_str));
        }

        // record our explicit effective shader_defs
        for line in source.lines() {
            if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                effective_defs.insert(def.as_str());
            }
            if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                effective_defs.insert(def.as_str());
            }
        }

        let effective_defs = effective_defs.iter().map(ToString::to_string).collect();

        let module_set = ComposableModuleDefinition {
            name: module_name.clone(),
            source,
            language,
            effective_defs,
            modules: Default::default(),
        };

        // todo invalidate dependent modules if this module already exists

        self.module_sets.insert(module_name.clone(), module_set);
        Ok(self.module_sets.get(&module_name).unwrap())
    }

    // shunt all data owned by a composable into a derived module
    fn add_composable_data<'a>(derived: &mut DerivedModule<'a>, composable: &'a ComposableModule) {
        derived.set_shader_source(&composable.module_ir);

        for (h, ty) in composable.module_ir.types.iter() {
            if let Some(name) = &ty.name {
                if composable.owned_types.contains(name) {
                    // println!("import type {}", name);
                    derived.import_type(&h);
                } else {
                    // println!("skip {}", name);
                }
            }
        }

        for (h, c) in composable.module_ir.constants.iter() {
            if let Some(name) = &c.name {
                if composable.owned_constants.contains(name) {
                    derived.import_const(&h);
                }
            }
        }

        for (h, v) in composable.module_ir.global_variables.iter() {
            if let Some(name) = &v.name {
                if composable.owned_vars.contains(name) {
                    derived.import_global(&h);
                }
            }
        }

        for (_, f) in composable.module_ir.functions.iter() {
            if let Some(name) = &f.name {
                if composable.owned_functions.contains(name) {
                    derived.import_function(f);
                }
            }
        }

        derived.clear_shader_source();
    }

    // add an import (and recursive imports) into a derived module
    fn add_import<'a>(
        &'a self,
        derived: &mut DerivedModule<'a>,
        import: &String,
        shader_defs: &HashSet<String>,
        already_added: &mut HashSet<String>,
    ) {
        if already_added.contains(import) {
            return;
        }
        already_added.insert(import.clone());

        let import_module_set = self.module_sets.get(import).unwrap();
        let module = import_module_set.get(shader_defs).unwrap();

        for import in module.imports.iter() {
            self.add_import(derived, import, shader_defs, already_added);
        }

        Self::add_composable_data(derived, module);
    }

    // build required ComposableModules for a given set of shader_defs
    fn ensure_imports(
        &mut self,
        imports: Vec<ImportDefinition>,
        shader_defs: &HashSet<String>,
        already_built: &mut HashSet<String>,
    ) -> Result<(), ComposerError> {
        let mut to_build = Vec::default();

        for ImportDefinition { import, .. } in imports.into_iter() {
            if already_built.contains(&import) {
                continue;
            }

            match self.module_sets.get(&import) {
                Some(module_set) => {
                    if module_set.get(shader_defs).is_none() {
                        // println!("planning to build {}:{:?}", import, shader_defs);
                        let (_, imports) =
                            self.preprocess_defs(&import, &module_set.source, shader_defs)?;
                        to_build.push((
                            import.clone(),
                            module_set.source.clone(),
                            module_set.language,
                            imports,
                        ));
                    } else {
                        // println!("already got {}:{:?}", import, shader_defs);
                        already_built.insert(import.clone());
                    }
                }
                None => return Err(ComposerError::ImportNotFound(import.clone())),
            }
        }

        for (name, source, language, imports) in to_build.into_iter() {
            if !already_built.insert(name.clone()) {
                continue;
            };

            // println!("> checking imports for {}", name);
            self.ensure_imports(imports, shader_defs, already_built)?;
            // println!("< done checking imports for {}", name);
            let module =
                self.create_composable_module(&decorate(&name), &source, language, shader_defs)?;
            // println!("adding compiled module {}:{:?}", name, shader_defs);
            self.module_sets
                .get_mut(&name)
                .unwrap()
                .insert(shader_defs, module);
        }

        Ok(())
    }

    // build a naga shader module
    pub fn make_naga_module(
        &mut self,
        source: String,
        language: ShaderLanguage,
        shader_defs: &[String],
    ) -> Result<naga::Module, ComposerError> {
        let shader_defs = shader_defs.iter().cloned().collect();
        let (_, imports) = self.preprocess_defs("", &source, &shader_defs)?;

        // println!("shader imports: {:?}", imports);

        self.ensure_imports(imports, &shader_defs, &mut Default::default())?;

        let composable = self.create_composable_module(
            "",
            &source,
            language,
            &shader_defs.iter().cloned().collect(),
        )?;

        let mut derived = DerivedModule::default();

        let mut already_added = Default::default();
        for import in composable.imports.iter() {
            // println!("adding {}", import);
            self.add_import(&mut derived, import, &shader_defs, &mut already_added);
            // println!("after {}:\n{:#?}", import, derived);
        }

        // println!("before adding main module: {:#?}", derived);

        Self::add_composable_data(&mut derived, &composable);

        // println!("after adding main module: {:#?}", derived);

        let mut entry_points = Vec::default();
        derived.set_shader_source(&composable.module_ir);
        for ep in composable.module_ir.entry_points.iter() {
            let mapped_func = derived.localize_function(&ep.function);
            entry_points.push(EntryPoint {
                name: ep.name.clone(),
                function: mapped_func,
                stage: ep.stage,
                early_depth_test: ep.early_depth_test,
                workgroup_size: ep.workgroup_size,
            });
        }

        // println!("entry points: {:#?}", entry_points);

        let naga_module = naga::Module {
            entry_points,
            ..derived.into()
        };

        // validation
        if self.validate {
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::default(),
            )
            .validate(&naga_module);
            match info {
                Ok(_) => Ok(naga_module),
                Err(e) => {
                    let mut inner = e.clone().into_inner();
                    let name = match &mut inner {
                        ValidationError::Type { name, .. } => name,
                        ValidationError::Constant { name, .. } => name,
                        ValidationError::GlobalVariable { name, .. } => name,
                        ValidationError::Function { name, .. } => name,
                        ValidationError::EntryPoint { name, .. } => name,
                        ValidationError::Layouter(_) => return Err(ComposerError::ShaderValidationError(e, naga_module)),
                        ValidationError::Corrupted => return Err(ComposerError::ShaderValidationError(e, naga_module)),
                    };

                    for (mod_name, _) in self.module_sets.iter() {
                        *name = name.replace(&decorate(mod_name), &format!("{}::", mod_name));
                    }

                    Err(ComposerError::ShaderValidationError(
                        WithSpan::new(inner),
                        naga_module,
                    ))
                }
            }
        } else {
            Ok(naga_module)
        }
    }
}
