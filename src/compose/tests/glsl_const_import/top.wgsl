#version 450

#import common

fn main() -> vec4<f32> { 
    return vec4(1.0, common::my_constant, 0.0, 1.0); 
}