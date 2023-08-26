use std::collections::{HashMap, HashSet};

use regex::Regex;
use tracing::warn;

use super::{
    comment_strip_iter::CommentReplaceExt, ComposerErrorInner, ImportDefWithOffset,
    ImportDefinition, ShaderDefValue,
};

#[derive(Debug)]
pub struct Preprocessor {
    version_regex: Regex,
    ifdef_regex: Regex,
    ifndef_regex: Regex,
    ifop_regex: Regex,
    else_regex: Regex,
    endif_regex: Regex,
    def_regex: Regex,
    def_regex_delimited: Regex,
    import_custom_path_as_regex: Regex,
    import_custom_path_regex: Regex,
    import_items_regex: Regex,
    identifier_regex: Regex,
    define_import_path_regex: Regex,
    define_shader_def_regex: Regex,
}

impl Default for Preprocessor {
    fn default() -> Self {
        Self {
            version_regex: Regex::new(r"^\s*#version\s+([0-9]+)").unwrap(),
            ifdef_regex: Regex::new(r"^\s*#\s*(else\s+)?\s*ifdef\s+([\w|\d|_]+)").unwrap(),
            ifndef_regex: Regex::new(r"^\s*#\s*(else\s+)?\s*ifndef\s+([\w|\d|_]+)").unwrap(),
            ifop_regex: Regex::new(
                r"^\s*#\s*(else\s+)?\s*if\s+([\w|\d|_]+)\s*([=!<>]*)\s*([-\w|\d]+)",
            )
            .unwrap(),
            else_regex: Regex::new(r"^\s*#\s*else").unwrap(),
            endif_regex: Regex::new(r"^\s*#\s*endif").unwrap(),
            def_regex: Regex::new(r"#\s*([\w|\d|_]+)").unwrap(),
            def_regex_delimited: Regex::new(r"#\s*\{([\w|\d|_]+)\}").unwrap(),
            import_custom_path_as_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)\s+as\s+([^\s]+)")
                .unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)").unwrap(),
            import_items_regex: Regex::new(
                r"^\s*#\s*import\s+([^\s]+)\s+((?:[\w|\d|_]+)(?:\s*,\s*[\w|\d|_]+)*)",
            )
            .unwrap(),
            identifier_regex: Regex::new(r"([\w|\d|_]+)").unwrap(),
            define_import_path_regex: Regex::new(r"^\s*#\s*define_import_path\s+([^\s]+)").unwrap(),
            define_shader_def_regex: Regex::new(r"^\s*#\s*define\s+([\w|\d|_]+)\s*([-\w|\d]+)?")
                .unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct PreprocessorMetaData {
    pub name: Option<String>,
    pub imports: Vec<ImportDefWithOffset>,
}

enum ScopeLevel {
    Active,           // conditions have been met
    PreviouslyActive, // conditions have previously been met
    NotActive,        // no conditions yet met
}

struct Scope(Vec<ScopeLevel>);

impl Scope {
    fn new() -> Self {
        Self(vec![ScopeLevel::Active])
    }

    fn branch(
        &mut self,
        is_else: bool,
        condition: bool,
        offset: usize,
    ) -> Result<(), ComposerErrorInner> {
        if is_else {
            let prev_scope = self.0.pop().unwrap();
            let parent_scope = self
                .0
                .last()
                .ok_or(ComposerErrorInner::ElseWithoutCondition(offset))?;
            let new_scope = if !matches!(parent_scope, ScopeLevel::Active) {
                ScopeLevel::NotActive
            } else if !matches!(prev_scope, ScopeLevel::NotActive) {
                ScopeLevel::PreviouslyActive
            } else if condition {
                ScopeLevel::Active
            } else {
                ScopeLevel::NotActive
            };

            self.0.push(new_scope);
        } else {
            let parent_scope = self.0.last().unwrap_or(&ScopeLevel::Active);
            let new_scope = if matches!(parent_scope, ScopeLevel::Active) && condition {
                ScopeLevel::Active
            } else {
                ScopeLevel::NotActive
            };

            self.0.push(new_scope);
        }

        Ok(())
    }

    fn pop(&mut self, offset: usize) -> Result<(), ComposerErrorInner> {
        self.0.pop();
        if self.0.is_empty() {
            Err(ComposerErrorInner::TooManyEndIfs(offset))
        } else {
            Ok(())
        }
    }

    fn active(&self) -> bool {
        matches!(self.0.last().unwrap(), ScopeLevel::Active)
    }

    fn finish(&self, offset: usize) -> Result<(), ComposerErrorInner> {
        if self.0.len() != 1 {
            Err(ComposerErrorInner::NotEnoughEndIfs(offset))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct PreprocessOutput {
    pub preprocessed_source: String,
    pub meta: PreprocessorMetaData,
}

impl Preprocessor {
    // process #if[(n)?def]? / #else / #endif preprocessor directives,
    // strip module name and imports
    // also strip "#version xxx"
    pub fn preprocess(
        &self,
        shader_str: &str,
        shader_defs: &HashMap<String, ShaderDefValue>,
        mut validate_len: bool,
    ) -> Result<PreprocessOutput, ComposerErrorInner> {
        let mut imports = Vec::new();

        let mut scope = Scope::new();
        let mut final_string = String::new();
        let mut name = None;
        let mut offset = 0;

        let mut at_start = true;

        #[cfg(debug)]
        let len = shader_str.len();

        // this code broadly stolen from bevy_render::ShaderProcessor
        for (line, original_line) in shader_str
            .lines()
            .replace_comments()
            .zip(shader_str.lines())
        {
            let line = &line;

            let mut output = false;
            let mut still_at_start = false;
            if line.is_empty() || line.chars().all(|c| c.is_ascii_whitespace()) {
                still_at_start = true;
            }

            if let Some(cap) = self.version_regex.captures(line) {
                let v = cap.get(1).unwrap().as_str();
                if v != "440" && v != "450" {
                    return Err(ComposerErrorInner::GlslInvalidVersion(offset));
                }
            } else if let Some(cap) = self.ifdef_regex.captures(line) {
                let is_else = cap.get(1).is_some();
                let cond = shader_defs.contains_key(cap.get(2).unwrap().as_str());
                scope.branch(is_else, cond, offset)?;
            } else if let Some(cap) = self.ifndef_regex.captures(line) {
                let is_else = cap.get(1).is_some();
                let cond = !shader_defs.contains_key(cap.get(2).unwrap().as_str());
                scope.branch(is_else, cond, offset)?;
            } else if let Some(cap) = self.ifop_regex.captures(line) {
                let is_else = cap.get(1).is_some();
                let def = cap.get(2).unwrap();
                let op = cap.get(3).unwrap();
                let val = cap.get(4).unwrap();

                fn act_on<T: Eq + Ord>(
                    a: T,
                    b: T,
                    op: &str,
                    pos: usize,
                ) -> Result<bool, ComposerErrorInner> {
                    match op {
                        "==" => Ok(a == b),
                        "!=" => Ok(a != b),
                        ">" => Ok(a > b),
                        ">=" => Ok(a >= b),
                        "<" => Ok(a < b),
                        "<=" => Ok(a <= b),
                        _ => Err(ComposerErrorInner::UnknownShaderDefOperator {
                            pos,
                            operator: op.to_string(),
                        }),
                    }
                }

                let def_value =
                    shader_defs
                        .get(def.as_str())
                        .ok_or(ComposerErrorInner::UnknownShaderDef {
                            pos: offset,
                            shader_def_name: def.as_str().to_string(),
                        })?;
                let new_scope = match def_value {
                    ShaderDefValue::Bool(def_value) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ComposerErrorInner::InvalidShaderDefComparisonValue {
                                pos: offset,
                                shader_def_name: def.as_str().to_string(),
                                value: val.as_str().to_string(),
                                expected: "bool".to_string(),
                            }
                        })?;
                        act_on(*def_value, val, op.as_str(), offset)?
                    }
                    ShaderDefValue::Int(def_value) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ComposerErrorInner::InvalidShaderDefComparisonValue {
                                pos: offset,
                                shader_def_name: def.as_str().to_string(),
                                value: val.as_str().to_string(),
                                expected: "int".to_string(),
                            }
                        })?;
                        act_on(*def_value, val, op.as_str(), offset)?
                    }
                    ShaderDefValue::UInt(def_value) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ComposerErrorInner::InvalidShaderDefComparisonValue {
                                pos: offset,
                                shader_def_name: def.as_str().to_string(),
                                value: val.as_str().to_string(),
                                expected: "int".to_string(),
                            }
                        })?;
                        act_on(*def_value, val, op.as_str(), offset)?
                    }
                };

                scope.branch(is_else, new_scope, offset)?;
            } else if self.else_regex.is_match(line) {
                scope.branch(true, true, offset)?;
            } else if self.endif_regex.is_match(line) {
                scope.pop(offset)?;
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                name = Some(cap.get(1).unwrap().as_str().to_string());
            } else if let Some(cap) = self.define_shader_def_regex.captures(line) {
                if at_start {
                    still_at_start = true;

                    let def = cap.get(1).unwrap();
                    let name = def.as_str().to_string();

                    let value = if let Some(val) = cap.get(2) {
                        if let Ok(val) = val.as_str().parse::<u32>() {
                            ShaderDefValue::UInt(val)
                        } else if let Ok(val) = val.as_str().parse::<i32>() {
                            ShaderDefValue::Int(val)
                        } else if let Ok(val) = val.as_str().parse::<bool>() {
                            ShaderDefValue::Bool(val)
                        } else {
                            return Err(ComposerErrorInner::InvalidShaderDefDefinitionValue {
                                name,
                                value: val.as_str().to_string(),
                                pos: offset,
                            });
                        }
                    } else {
                        ShaderDefValue::Bool(true)
                    };

                    match shader_defs.get(name.as_str()) {
                        Some(current_value) if *current_value == value => (),
                        _ => return Err(ComposerErrorInner::DefineInModule(offset)),
                    }
                } else {
                    return Err(ComposerErrorInner::DefineInModule(offset));
                }
            } else if scope.active() {
                if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                    imports.push(ImportDefWithOffset {
                        definition: ImportDefinition {
                            import: cap.get(1).unwrap().as_str().to_string(),
                            as_name: Some(cap.get(2).unwrap().as_str().to_string()),
                            items: Default::default(),
                        },
                        offset,
                    });
                } else if let Some(cap) = self.import_items_regex.captures(line) {
                    imports.push(ImportDefWithOffset {
                        definition: ImportDefinition {
                            import: cap.get(1).unwrap().as_str().to_string(),
                            as_name: None,
                            items: Some(
                                self.identifier_regex
                                    .captures_iter(cap.get(2).unwrap().as_str())
                                    .map(|ident_cap| ident_cap.get(1).unwrap().as_str().to_owned())
                                    .collect(),
                            ),
                        },
                        offset,
                    });
                } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                    imports.push(ImportDefWithOffset {
                        definition: ImportDefinition {
                            import: cap.get(1).unwrap().as_str().to_string(),
                            as_name: None,
                            items: Default::default(),
                        },
                        offset,
                    });
                } else {
                    let mut line_with_defs = original_line.to_string();
                    for capture in self.def_regex.captures_iter(line) {
                        let def = capture.get(1).unwrap();
                        if let Some(def) = shader_defs.get(def.as_str()) {
                            line_with_defs = self
                                .def_regex
                                .replace(&line_with_defs, def.value_as_string())
                                .to_string();
                        }
                    }
                    for capture in self.def_regex_delimited.captures_iter(line) {
                        let def = capture.get(1).unwrap();
                        if let Some(def) = shader_defs.get(def.as_str()) {
                            line_with_defs = self
                                .def_regex_delimited
                                .replace(&line_with_defs, def.value_as_string())
                                .to_string();
                        }
                    }
                    final_string.push_str(&line_with_defs);
                    let diff = line.len() as i32 - line_with_defs.len() as i32;
                    if diff > 0 {
                        final_string.extend(std::iter::repeat(" ").take(diff as usize));
                    } else if diff < 0 && validate_len {
                        // this sucks
                        warn!("source code map requires shader_def values to be no longer than the corresponding shader_def name, error reporting may not be correct:\noriginal: {}\nreplaced: {}", line, line_with_defs);
                        validate_len = false;
                    }
                    output = true;
                }
            }

            if !output {
                // output spaces for removed lines to keep spans consistent (errors report against substituted_source, which is not preprocessed)
                final_string.extend(std::iter::repeat(" ").take(line.len()));
            }
            final_string.push('\n');
            offset += line.len() + 1;

            at_start &= still_at_start;
        }

        scope.finish(offset)?;

        #[cfg(debug)]
        if validate_len {
            let revised_len = final_string.len();
            assert_eq!(len, revised_len);
        }

        Ok(PreprocessOutput {
            preprocessed_source: final_string,
            meta: PreprocessorMetaData { name, imports },
        })
    }

    // extract module name and all possible imports
    pub fn get_preprocessor_metadata(
        &self,
        shader_str: &str,
        allow_defines: bool,
    ) -> Result<(PreprocessorMetaData, HashMap<String, ShaderDefValue>), ComposerErrorInner> {
        let mut imports = Vec::new();
        let mut name = None;
        let mut offset = 0;
        let mut defines = HashMap::default();

        for line in shader_str.lines().replace_comments() {
            let line = &line;
            if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                imports.push(ImportDefWithOffset {
                    definition: ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: Some(cap.get(2).unwrap().as_str().to_string()),
                        items: Default::default(),
                    },
                    offset,
                });
            } else if let Some(cap) = self.import_items_regex.captures(line) {
                imports.push(ImportDefWithOffset {
                    definition: ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: None,
                        items: Some(
                            self.identifier_regex
                                .captures_iter(cap.get(2).unwrap().as_str())
                                .map(|ident_cap| ident_cap.get(1).unwrap().as_str().to_owned())
                                .collect(),
                        ),
                    },
                    offset,
                });
            } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                imports.push(ImportDefWithOffset {
                    definition: ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: None,
                        items: Default::default(),
                    },
                    offset,
                });
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                name = Some(cap.get(1).unwrap().as_str().to_string());
            } else if let Some(cap) = self.define_shader_def_regex.captures(line) {
                if allow_defines {
                    let def = cap.get(1).unwrap();
                    let name = def.as_str().to_string();

                    let value = if let Some(val) = cap.get(2) {
                        if let Ok(val) = val.as_str().parse::<u32>() {
                            ShaderDefValue::UInt(val)
                        } else if let Ok(val) = val.as_str().parse::<i32>() {
                            ShaderDefValue::Int(val)
                        } else if let Ok(val) = val.as_str().parse::<bool>() {
                            ShaderDefValue::Bool(val)
                        } else {
                            ShaderDefValue::Bool(false) // this error will get picked up when we fully preprocess the module
                        }
                    } else {
                        ShaderDefValue::Bool(true)
                    };

                    defines.insert(name, value);
                } else {
                    return Err(ComposerErrorInner::DefineInModule(offset));
                }
            }

            offset += line.len() + 1;
        }

        Ok((PreprocessorMetaData { name, imports }, defines))
    }

    pub fn effective_defs(&self, source: &str) -> HashSet<String> {
        let mut effective_defs = HashSet::default();

        for line in source.lines().replace_comments() {
            if let Some(cap) = self.ifdef_regex.captures(&line) {
                let def = cap.get(2).unwrap();
                effective_defs.insert(def.as_str().to_owned());
            }
            if let Some(cap) = self.ifndef_regex.captures(&line) {
                let def = cap.get(2).unwrap();
                effective_defs.insert(def.as_str().to_owned());
            }
            if let Some(cap) = self.ifop_regex.captures(&line) {
                let def = cap.get(2).unwrap();
                effective_defs.insert(def.as_str().to_owned());
            }
        }

        effective_defs
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[rustfmt::skip]
    const WGSL_ELSE_IFDEF: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#ifdef TEXTURE
// Main texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else ifdef SECOND_TEXTURE
// Second texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else ifdef THIRD_TEXTURE
// Third texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else
@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    //preprocessor tests
    #[test]
    fn process_shader_def_unknown_operator() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
#if TEXTURE !! true
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        let processor = Preprocessor::default();

        let result_missing = processor.preprocess(
            WGSL,
            &[("TEXTURE".to_owned(), ShaderDefValue::Bool(true))].into(),
            true,
        );

        let expected: Result<Preprocessor, ComposerErrorInner> =
            Err(ComposerErrorInner::UnknownShaderDefOperator {
                pos: 124,
                operator: "!!".to_string(),
            });

        assert_eq!(format!("{result_missing:?}"), format!("{expected:?}"),);
    }
    #[test]
    fn process_shader_def_equal_int() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
#if TEXTURE == 3
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_EQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
                
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
      
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_NEQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
                
                     
                                    
      
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result_eq = processor
            .preprocess(
                WGSL,
                &[("TEXTURE".to_string(), ShaderDefValue::Int(3))].into(),
                true,
            )
            .unwrap();
        assert_eq!(result_eq.preprocessed_source, EXPECTED_EQ);

        let result_neq = processor
            .preprocess(
                WGSL,
                &[("TEXTURE".to_string(), ShaderDefValue::Int(7))].into(),
                true,
            )
            .unwrap();
        assert_eq!(result_neq.preprocessed_source, EXPECTED_NEQ);

        let result_missing = processor.preprocess(WGSL, &Default::default(), true);

        let expected_err: Result<
            (Option<String>, String, Vec<ImportDefWithOffset>),
            ComposerErrorInner,
        > = Err(ComposerErrorInner::UnknownShaderDef {
            pos: 124,
            shader_def_name: "TEXTURE".to_string(),
        });
        assert_eq!(format!("{result_missing:?}"), format!("{expected_err:?}"),);

        let result_wrong_type = processor.preprocess(
            WGSL,
            &[("TEXTURE".to_string(), ShaderDefValue::Bool(true))].into(),
            true,
        );

        let expected_err: Result<
            (Option<String>, String, Vec<ImportDefWithOffset>),
            ComposerErrorInner,
        > = Err(ComposerErrorInner::InvalidShaderDefComparisonValue {
            pos: 124,
            shader_def_name: "TEXTURE".to_string(),
            expected: "bool".to_string(),
            value: "3".to_string(),
        });

        assert_eq!(
            format!("{result_wrong_type:?}"),
            format!("{expected_err:?}")
        );
    }

    #[test]
    fn process_shader_def_equal_bool() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
#if TEXTURE == true
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_EQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
                   
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
      
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_NEQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
                   
                     
                                    
      
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result_eq = processor
            .preprocess(
                WGSL,
                &[("TEXTURE".to_string(), ShaderDefValue::Bool(true))].into(),
                true,
            )
            .unwrap();
        assert_eq!(result_eq.preprocessed_source, EXPECTED_EQ);

        let result_neq = processor
            .preprocess(
                WGSL,
                &[("TEXTURE".to_string(), ShaderDefValue::Bool(false))].into(),
                true,
            )
            .unwrap();
        assert_eq!(result_neq.preprocessed_source, EXPECTED_NEQ);
    }

    #[test]
    fn process_shader_def_not_equal_bool() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
#if TEXTURE != false
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_EQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
                    
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
      
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_NEQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
                    
                     
                                    
      
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result_eq = processor
            .preprocess(
                WGSL,
                &[("TEXTURE".to_string(), ShaderDefValue::Bool(true))].into(),
                true,
            )
            .unwrap();
        assert_eq!(result_eq.preprocessed_source, EXPECTED_EQ);

        let result_neq = processor
            .preprocess(
                WGSL,
                &[("TEXTURE".to_string(), ShaderDefValue::Bool(false))].into(),
                true,
            )
            .unwrap();
        assert_eq!(result_neq.preprocessed_source, EXPECTED_NEQ);

        let result_missing = processor.preprocess(WGSL, &[].into(), true);
        let expected_err: Result<
            (Option<String>, String, Vec<ImportDefWithOffset>),
            ComposerErrorInner,
        > = Err(ComposerErrorInner::UnknownShaderDef {
            pos: 124,
            shader_def_name: "TEXTURE".to_string(),
        });
        assert_eq!(format!("{result_missing:?}"), format!("{expected_err:?}"),);

        let result_wrong_type = processor.preprocess(
            WGSL,
            &[("TEXTURE".to_string(), ShaderDefValue::Int(7))].into(),
            true,
        );

        let expected_err: Result<
            (Option<String>, String, Vec<ImportDefWithOffset>),
            ComposerErrorInner,
        > = Err(ComposerErrorInner::InvalidShaderDefComparisonValue {
            pos: 124,
            shader_def_name: "TEXTURE".to_string(),
            expected: "int".to_string(),
            value: "false".to_string(),
        });
        assert_eq!(
            format!("{result_wrong_type:?}"),
            format!("{expected_err:?}"),
        );
    }

    #[test]
    fn process_shader_def_replace() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    var a: i32 = #FIRST_VALUE;
    var b: i32 = #FIRST_VALUE * #SECOND_VALUE;
    var c: i32 = #MISSING_VALUE;
    var d: bool = #BOOL_VALUE;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_REPLACED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    var a: i32 = 5;           
    var b: i32 = 5 * 3;                       
    var c: i32 = #MISSING_VALUE;
    var d: bool = true;       
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(
                WGSL,
                &[
                    ("BOOL_VALUE".to_string(), ShaderDefValue::Bool(true)),
                    ("FIRST_VALUE".to_string(), ShaderDefValue::Int(5)),
                    ("SECOND_VALUE".to_string(), ShaderDefValue::Int(3)),
                ]
                .into(),
                true,
            )
            .unwrap();
        assert_eq!(result.preprocessed_source, EXPECTED_REPLACED);
    }

    #[test]
    fn process_shader_define_in_shader() {
        #[rustfmt::skip]
        const WGSL: &str = r"
#define NOW_DEFINED
#ifdef NOW_DEFINED
defined
#endif
";

        #[rustfmt::skip]
        const EXPECTED: &str = r"
                   
                  
defined
      
";
        let processor = Preprocessor::default();
        let (_, defines) = processor.get_preprocessor_metadata(&WGSL, true).unwrap();
        println!("defines: {:?}", defines);
        let result = processor.preprocess(&WGSL, &defines, true).unwrap();
        assert_eq!(result.preprocessed_source, EXPECTED);
    }

    #[test]
    fn process_shader_define_in_shader_with_value() {
        #[rustfmt::skip]
        const WGSL: &str = r"
#define DEFUINT 1
#define DEFINT -1
#define DEFBOOL false
#if DEFUINT == 1
uint: #DEFUINT
#endif
#if DEFINT == -1
int: #DEFINT
#endif
#if DEFBOOL == false
bool: #DEFBOOL
#endif
";

        #[rustfmt::skip]
        const EXPECTED: &str = r"
                 
                 
                     
                
uint: 1       
      
                
int: -1     
      
                    
bool: false   
      
";
        let processor = Preprocessor::default();
        let (_, defines) = processor.get_preprocessor_metadata(&WGSL, true).unwrap();
        println!("defines: {:?}", defines);
        let result = processor.preprocess(&WGSL, &defines, true).unwrap();
        assert_eq!(result.preprocessed_source, EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_else() {
        #[rustfmt::skip]
        const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(&WGSL_ELSE_IFDEF, &[].into(), true)
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_ifdef_no_match_and_no_fallback_else() {
        #[rustfmt::skip]
        const WGSL_ELSE_IFDEF_NO_ELSE_FALLBACK: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#ifdef TEXTURE
// Main texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else ifdef OTHER_TEXTURE
// Other texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(&WGSL_ELSE_IFDEF_NO_ELSE_FALLBACK, &[].into(), true)
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_first_clause() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
              
// Main texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
                          
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(
                &WGSL_ELSE_IFDEF,
                &[("TEXTURE".to_string(), ShaderDefValue::Bool(true))].into(),
                true,
            )
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_second_clause() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
// Second texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(
                &WGSL_ELSE_IFDEF,
                &[("SECOND_TEXTURE".to_string(), ShaderDefValue::Bool(true))].into(),
                true,
            )
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_third_clause() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
// Third texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(
                &WGSL_ELSE_IFDEF,
                &[("THIRD_TEXTURE".to_string(), ShaderDefValue::Bool(true))].into(),
                true,
            )
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_ifdef_only_accepts_one_valid_else_ifdef() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;
// Second texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(
                &WGSL_ELSE_IFDEF,
                &[
                    ("SECOND_TEXTURE".to_string(), ShaderDefValue::Bool(true)),
                    ("THIRD_TEXTURE".to_string(), ShaderDefValue::Bool(true)),
                ]
                .into(),
                true,
            )
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_ifdef_complicated_nesting() {
        // Test some nesting including #else ifdef statements
        // 1. Enter an #else ifdef
        // 2. Then enter an #else
        // 3. Then enter another #else ifdef

        #[rustfmt::skip]
        const WGSL_COMPLICATED_ELSE_IFDEF: &str = r"
#ifdef NOT_DEFINED
// not defined
#else ifdef IS_DEFINED
// defined 1
#ifdef NOT_DEFINED
// not defined
#else
// should be here
#ifdef NOT_DEFINED
// not defined
#else ifdef ALSO_NOT_DEFINED
// not defined
#else ifdef IS_DEFINED
// defined 2
#endif
#endif
#endif
";

        #[rustfmt::skip]
        const EXPECTED: &str = r"
// defined 1
// should be here
// defined 2
";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(
                &WGSL_COMPLICATED_ELSE_IFDEF,
                &[("IS_DEFINED".to_string(), ShaderDefValue::Bool(true))].into(),
                true,
            )
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_ifndef() {
        #[rustfmt::skip]
        const INPUT: &str = r"
#ifdef NOT_DEFINED
fail 1
#else ifdef ALSO_NOT_DEFINED
fail 2
#else ifndef ALSO_ALSO_NOT_DEFINED
ok
#else
fail 3
#endif
";

        const EXPECTED: &str = r"ok";
        let processor = Preprocessor::default();
        let result = processor.preprocess(&INPUT, &[].into(), true).unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }

    #[test]
    fn process_shader_def_else_if() {
        #[rustfmt::skip]
        const INPUT: &str = r"
#ifdef NOT_DEFINED
fail 1
#else if x == 1
fail 2
#else if x == 2
ok
#else
fail 3
#endif
";

        const EXPECTED: &str = r"ok";
        let processor = Preprocessor::default();
        let result = processor
            .preprocess(
                &INPUT,
                &[("x".to_owned(), ShaderDefValue::Int(2))].into(),
                true,
            )
            .unwrap();
        assert_eq!(
            result
                .preprocessed_source
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", ""),
            EXPECTED
                .replace(" ", "")
                .replace("\n", "")
                .replace("\r", "")
        );
    }
}
