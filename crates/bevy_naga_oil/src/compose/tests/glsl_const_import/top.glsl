#version 450

#import common

out vec4 out_color;

void main() { 
    out_color = vec4(1, common::my_constant, 0, 1); 
}