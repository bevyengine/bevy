Naga Organised Integration Library (`naga-oil`) is a crate for combining and manipulating shaders.

- `compose` presents a modular shader composition framework
- `prune` strips shaders down to required parts

and probably less useful externally:
- `derive` allows importing of items from multiple shaders into a single shader
- `redirect` modifies a shader by substituting function calls and modifying bindings

# Compose

the compose module allows construction of shaders from modules (which are themselves shaders).

it does this by treating shaders as modules, and 
- building each module independently to naga IR
- creating "header" files for each supported language, which are used to build dependent modules/shaders
- making final shaders by combining the shader IR with the IR for imported modules

for multiple small shaders with large common imports, this can be faster than parsing the full source for each shader, and it allows for constructing shaders in a cleaner modular manner with better scope control.

## imports

shaders can be added to the composer as modules. this makes their types, constants, variables and functions available to modules/shaders that import them. note that importing a module will affect the final shader's global state if the module defines globals variables with bindings.

modules may include a `#define_import_path` directive that names the module:

```wgsl
#define_import_path my_module

fn my_func() -> f32 {
	return 1.0;
}
```

shaders can then import the module with an `#import` directive (with an optional `as` name). at point of use, imported items must be qualified:

```wgsl
#import my_module
#import my_other_module as Mod2

fn main() -> f32 {
    let x = my_module::my_func();
    let y = Mod2::my_other_func();
    return x*y;
}
```

or import a comma-separated list of individual items with a `#from` directive. at point of use, imported items must be prefixed with `::` :

```wgsl
#from my_module import my_func, my_const

fn main() -> f32 {
    return ::my_func(::my_const);
}
```

imports can be nested - modules may import other modules, but not recursively. when a new module is added, all its `#import`s must already have been added.
the same module can be imported multiple times by different modules in the import tree.
there is no overlap of namespaces, so the same function names (or type, constant, or variable names) may be used in different modules.

note: when importing an item with the `#from` directive, the final shader will include the required dependencies (bindings, globals, consts, other functions) of the imported item, but will not include the rest of the imported module. it will however still include all of any modules imported by the imported module. this is probably not desired in general and may be fixed in a future version. currently for a more complete culling of unused dependencies the `prune` module can be used.

## overriding functions

functions defined in imported modules can be overridden using the `override` keyword:

```wgsl
#import bevy_pbr::lighting as Lighting

override fn Lighting::point_light (world_position: vec3<f32>) -> vec3<f32> {
    let original = Lighting::point_light(world_position);
    let quantized = vec3<u32>(original * 3.0);
    return vec3<f32>(quantized) / 3.0;
}
```

override function definitions cause *all* calls to the original function in the entire shader scope to be replaced by calls to the new function, with the exception of calls within the override function itself.

the function signature of the override must match the base function. 

overrides can be specified at any point in the final shader's import tree. 

multiple overrides can be applied to the same function. for example, given :
- a module `a` containing a function `f`, 
- a module `b` that imports `a`, and containing an `override a::f` function, 
- a module `c` that imports `a` and `b`, and containing an `override a::f` function,

then `b` and `c` both specify an override for `a::f`. 

the `override fn a::f` declared in module `b` may call to `a::f` within its body.

the `override fn a::f` declared in module `c` may call to `a::f` within its body, but the call will be redirected to `b::f`.

any other calls to `a::f` (within modules `a` or `b`, or anywhere else) will end up redirected to `c::f`.

in this way a chain or stack of overrides can be applied.

different overrides of the same function can be specified in different import branches. the final stack will be ordered based on the first occurrence of the override in the import tree (using a depth first search). 

note that imports into a module/shader are processed in order, but are processed before the body of the current shader/module regardless of where they occur in that module, so there is no way to import a module containing an override and inject a call into the override stack prior to that imported override. you can instead create two modules each containing an override and import them into a parent module/shader to order them as required.

override functions can currently only be defined in wgsl.

## languages

modules can we written in GLSL or WGSL. shaders with entry points can be imported as modules (provided they have a `#define_import_path` directive). entry points are available to call from imported modules either via their name (for WGSL) or via `module::main` (for GLSL).

final shaders can also be written in GLSL or WGSL. for GLSL users must specify whether the shader is a vertex shader or fragment shader via the ShaderType argument (GLSL compute shaders are not supported).

## preprocessing

when generating a final shader or adding a composable module, a set of `shader_def` string/value pairs must be provided. The value can be a bool (`ShaderDefValue::Bool`), an i32 (`ShaderDefValue::Int`) or a u32 (`ShaderDefValue::UInt`).

these allow conditional compilation of parts of modules and the final shader. conditional compilation is performed with `#if` / `#ifdef` / `#ifndef`, `#else` and `#endif` preprocessor directives:

```wgsl
fn get_number() -> f32 {
    #ifdef BIG_NUMBER
        return 999.0;
    #else
        return 0.999;
    #endif
}
```
the `#ifdef` directive matches when the def name exists in the input binding set (regardless of value). the `#ifndef` directive is the reverse.

the `#if` directive requires a def name, an operator, and a value for comparison:
- the def name must be a provided `shader_def` name. 
- the operator must be one of `==`, `!=`, `>=`, `>`, `<`, `<=`
- the value must be an integer literal if comparing to a `ShaderDef::Int`, or `true` or `false` if comparing to a `ShaderDef::Bool`.

shader defs can also be used in the shader source with `#SHADER_DEF` or `#{SHADER_DEF}`, and will be substituted for their value.

## error reporting

codespan reporting for errors is available using the error `emit_to_string` method. this requires validation to be enabled, which is true by default. `Composer::non_validating()` produces a non-validating composer that is not able to give accurate error reporting.

# prune

- strips dead code and bindings from shaders based on specified required output. intended to be used for building reduced depth and/or normal shaders from arbitrary vertex/fragment shaders.

proper docs tbd

# redirect

- redirects function calls
- wip: rebinds global bindings
- todo one day: translate between uniform, texture and buffer accesses so shaders written for direct passes can be used in indirect

proper docs tbd

# derive

- builds a single self-contained naga module out of parts of one or more existing modules

proper docs tbd
