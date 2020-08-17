use std::{borrow::Cow, collections::HashMap};

#[derive(Debug, Copy, Clone)]
pub enum IncludePath<'a> {
    Absolute(&'a str),
    Relative(&'a str),
}

pub fn preprocess(
    source: &str,
    includes: impl Fn(IncludePath) -> Result<String, ()>,
    macros: impl Fn(&str) -> Option<Option<String>>,
) -> Result<String, String> {
    let mut runtime_defines = HashMap::new();
    let mut stack = Vec::new();

    preprocess_internal(source, &includes, &macros, &mut runtime_defines, &mut stack)
}

fn preprocess_internal(
    source: &str,
    includes: &dyn Fn(IncludePath) -> Result<String, ()>,
    macros: &dyn Fn(&str) -> Option<Option<String>>,
    runtime_defines: &mut HashMap<String, Option<Option<String>>>,
    stack: &mut Vec<bool>,
) -> Result<String, String> {
    source
        .lines()
        .map(|line| -> Result<Option<Cow<str>>, String> {
            Ok(if let Some(directive) = line.strip_prefix('#') {
                let directive = directive.trim_start();
                let (directive, args) = if let Some(index) = directive.find(char::is_whitespace) {
                    let (directive, args) = directive.split_at(index);
                    (directive, Some(args.trim()))
                } else {
                    (directive, None)
                };

                match (directive, args) {
                    ("version", Some(version)) if version.parse::<u32>().is_ok() => {
                        Some(line.into())
                    }
                    ("ifdef", Some(definition)) => {
                        stack.push(match runtime_defines.get(definition) {
                            Some(Some(_)) => true,
                            None if macros(definition).is_some() => true,
                            Some(None) | None => false,
                        });
                        None
                    }
                    ("else", None) => {
                        if let Some(if_true) = stack.last_mut() {
                            *if_true = !*if_true;
                        } else {
                            return Err("inbalanced #else".to_string());
                        }
                        None
                    }
                    ("endif", None) => {
                        if stack.pop().is_none() {
                            return Err("unbalanced #endif".to_string());
                        }
                        None
                    }
                    ("include", Some(include_path)) => {
                        let include_path = if let Some(path) = include_path
                            .strip_prefix('<')
                            .and_then(|s| s.strip_suffix('>'))
                        {
                            IncludePath::Absolute(path)
                        } else if let Some(path) = include_path
                            .strip_prefix('\"')
                            .and_then(|s| s.strip_suffix('\"'))
                        {
                            IncludePath::Relative(path)
                        } else {
                            return Err(format!(
                                "\"{}\" not a valid format for the `#include` directive",
                                include_path
                            ));
                        };

                        let contents = includes(include_path).map_err(|_| {
                            format!("{:?} is not an includable shader", include_path)
                        })?;

                        let before_stack_len = stack.len();

                        let contents = preprocess_internal(
                            &contents,
                            includes,
                            macros,
                            runtime_defines,
                            stack,
                        )?;

                        if before_stack_len != stack.len() {
                            return Err(format!(
                                "unbalanced #ifdef, #else, #endif, etc in shader: {:?}",
                                include_path
                            ));
                        }

                        Some(contents.into())
                    }
                    ("define", Some(definition)) => {
                        let (name, value) =
                            if let Some(index) = definition.find(char::is_whitespace) {
                                let (name, value) = definition.split_at(index);
                                (name, Some(value.trim().to_string()))
                            } else {
                                (definition, None)
                            };
                        runtime_defines.insert(name.to_string(), Some(value));
                        None
                    }
                    ("undef", Some(name)) => {
                        if runtime_defines.remove_entry(name).is_none() {
                            return Err("cannot undefine definition that was not already defined"
                                .to_string());
                        }

                        if let Some(definition) = runtime_defines.get_mut(name) {
                            *definition = None;
                        }

                        None
                    }
                    (_, _) => return Err(format!("unknown directive: \"{}\"", line)),
                }
            } else if let Some(false) = stack.last() {
                None
            } else {
                Some(line.into())
            })
        })
        .filter_map(|res_opt| res_opt.transpose())
        .collect::<Result<Vec<_>, _>>()
        .map(|s| s.join("\n"))
}
