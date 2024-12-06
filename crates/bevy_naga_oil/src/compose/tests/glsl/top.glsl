#version 450

#import wgsl_module



void main() {
    gl_Position = vec4(wgsl_module::wgsl_func());
}