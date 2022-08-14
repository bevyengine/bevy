#version 450

#import wgsl_module



layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = vec4(wgsl_module::wgsl_func());
}